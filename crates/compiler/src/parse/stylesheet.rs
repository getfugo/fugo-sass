use indexmap::{IndexMap, IndexSet};
use std::{
    cell::Cell,
    collections::HashSet,
    ffi::OsString,
    mem,
    path::{Path, PathBuf},
    sync::Arc,
};

use codemap::{Span, Spanned};

use crate::{
    ContextFlags, Options, Token,
    ast::*,
    common::{Identifier, QuoteKind, unvendor},
    error::SassResult,
    lexer::Lexer,
    utils::{is_name, is_name_start, is_plain_css_import, opposite_bracket},
};

use super::{
    BaseParser, DeclarationOrBuffer, IdentifierParser, RESERVED_IDENTIFIERS, ScssParser,
    TokenParser, VariableDeclOrInterpolation,
    value::{Predicate, ValueParser},
};

mod callables;
mod control_rules;
mod css_if;
mod declarations;
mod expressions;
mod imports;
mod interpolation;
mod media;
mod modules;
mod raw_values;
mod statements;
mod supports;
mod variables;

// The parsing methods that no syntax overrides, as extension traits of `StylesheetParser`.
pub(crate) use callables::CallableParser;
pub(crate) use control_rules::ControlRuleParser;
pub(crate) use css_if::CssIfParser;
pub(crate) use declarations::DeclarationParser;
pub(crate) use expressions::ExpressionParser;
pub(crate) use imports::ImportParser;
pub(crate) use interpolation::InterpolationParser;
pub(crate) use media::MediaParser;
pub(crate) use modules::ModuleParser;
pub(crate) use raw_values::RawValueParser;
pub(crate) use statements::StatementParser;
pub(crate) use supports::SupportsParser;
pub(crate) use variables::VariableParser;

/// Default implementations are oriented towards the SCSS syntax, as both CSS and
/// SCSS share the behavior
pub(crate) trait StylesheetParser<'a>: BaseParser + Sized {
    // todo: make constant?
    fn is_plain_css(&self) -> bool;
    // todo: make constant?
    fn is_indented(&self) -> bool;
    fn options(&self) -> &Options<'_>;
    fn path(&self) -> &Path;
    fn empty_span(&self) -> Span;
    fn current_indentation(&self) -> usize;
    fn flags(&self) -> &ContextFlags;
    fn flags_mut(&mut self) -> &mut ContextFlags;

    #[allow(clippy::type_complexity)]
    const IDENTIFIER_LIKE: Option<fn(&mut Self) -> SassResult<Spanned<AstExpr>>> = None;

    fn parse_style_rule_selector(&mut self) -> SassResult<Interpolation> {
        self.almost_any_value(false)
    }

    fn expect_statement_separator(&mut self, _name: Option<&str>) -> SassResult<()> {
        self.whitespace_without_comments();
        match self.toks().peek() {
            Some(Token {
                kind: ';' | '}', ..
            })
            | None => Ok(()),
            _ => {
                self.expect_char(';')?;
                unreachable!();
            }
        }
    }

    fn at_end_of_statement(&self) -> bool {
        matches!(
            self.toks().peek(),
            Some(Token {
                kind: ';' | '}' | '{',
                ..
            }) | None
        )
    }

    fn looking_at_children(&mut self) -> SassResult<bool> {
        Ok(matches!(self.toks().peek(), Some(Token { kind: '{', .. })))
    }

    fn scan_else(&mut self, _if_indentation: usize) -> SassResult<bool> {
        let start = self.toks().cursor();

        self.whitespace()?;

        if self.scan_char('@') {
            if self.scan_identifier("else", true)? {
                return Ok(true);
            }

            if self.scan_identifier("elseif", true)? {
                // todo: deprecation warning here
                let new_cursor = self.toks().cursor() - 2;
                self.toks_mut().set_cursor(new_cursor);
                return Ok(true);
            }
        }

        self.toks_mut().set_cursor(start);

        Ok(false)
    }

    fn parse_children(
        &mut self,
        child: fn(&mut Self) -> SassResult<AstStmt>,
    ) -> SassResult<Vec<AstStmt>> {
        self.expect_char('{')?;
        self.whitespace_without_comments();
        let mut children = Vec::new();

        let mut found_matching_brace = false;

        while let Some(tok) = self.toks().peek() {
            match tok.kind {
                '$' => children.push(AstStmt::VariableDecl(
                    self.parse_variable_declaration_without_namespace(None, None)?,
                )),
                '/' => match self.toks().peek_n(1) {
                    Some(Token { kind: '/', .. }) => {
                        children.push(self.parse_silent_comment()?);
                        self.whitespace_without_comments();
                    }
                    Some(Token { kind: '*', .. }) => {
                        children.push(AstStmt::LoudComment(self.parse_loud_comment()?));
                        self.whitespace_without_comments();
                    }
                    _ => children.push(child(self)?),
                },
                ';' => {
                    self.toks_mut().next();
                    self.whitespace_without_comments();
                }
                '}' => {
                    self.expect_char('}')?;
                    found_matching_brace = true;
                    break;
                }
                _ => children.push(child(self)?),
            }
        }

        if !found_matching_brace {
            return Err(("expected \"}\".", self.toks().current_span()).into());
        }

        Ok(children)
    }

    fn parse_statements(
        &mut self,
        statement: fn(&mut Self) -> SassResult<Option<AstStmt>>,
    ) -> SassResult<Vec<AstStmt>> {
        let mut stmts = Vec::new();
        self.whitespace_without_comments();
        while let Some(tok) = self.toks().peek() {
            match tok.kind {
                '$' => stmts.push(AstStmt::VariableDecl(
                    self.parse_variable_declaration_without_namespace(None, None)?,
                )),
                '/' => match self.toks().peek_n(1) {
                    Some(Token { kind: '/', .. }) => {
                        stmts.push(self.parse_silent_comment()?);
                        self.whitespace_without_comments();
                    }
                    Some(Token { kind: '*', .. }) => {
                        stmts.push(AstStmt::LoudComment(self.parse_loud_comment()?));
                        self.whitespace_without_comments();
                    }
                    _ => {
                        if let Some(stmt) = statement(self)? {
                            stmts.push(stmt);
                        }
                    }
                },
                ';' => {
                    self.toks_mut().next();
                    self.whitespace_without_comments();
                }
                _ => {
                    if let Some(stmt) = statement(self)? {
                        stmts.push(stmt);
                    }
                }
            }
        }

        Ok(stmts)
    }

    // todo: rename
    fn __parse(&mut self) -> SassResult<StyleSheet> {
        let mut style_sheet = StyleSheet::new(
            self.is_plain_css(),
            self.options()
                .fs
                .canonicalize(self.path())
                .unwrap_or_else(|_| self.path().to_path_buf()),
        );

        // Allow a byte-order mark at the beginning of the document.
        self.scan_char('\u{feff}');

        style_sheet.body = self.parse_statements(|parser| {
            if parser.next_matches("@charset") {
                parser.expect_char('@')?;
                parser.expect_identifier("charset", false)?;
                parser.whitespace()?;
                parser.parse_string()?;
                return Ok(None);
            }

            Ok(Some(parser.parse_statement()?))
        })?;

        for (idx, child) in style_sheet.body.iter().enumerate() {
            match child {
                AstStmt::VariableDecl(_) | AstStmt::LoudComment(_) | AstStmt::SilentComment(_) => {
                    continue;
                }
                AstStmt::Use(..) => style_sheet.uses.push(idx),
                AstStmt::Forward(..) => style_sheet.forwards.push(idx),
                _ => break,
            }
        }

        Ok(style_sheet)
    }

    fn parse_at_rule(
        &mut self,
        child: fn(&mut Self) -> SassResult<AstStmt>,
    ) -> SassResult<AstStmt> {
        let start = self.toks().cursor();

        self.expect_char('@')?;
        let name = self.parse_interpolated_identifier()?;
        self.whitespace()?;

        // We want to set [_isUseAllowed] to `false` *unless* we're parsing
        // `@charset`, `@forward`, or `@use`. To avoid double-comparing the rule
        // name, we always set it to `false` and then set it back to its previous
        // value if we're parsing an allowed rule.
        let was_use_allowed = self.flags().is_use_allowed();
        self.flags_mut().set(ContextFlags::IS_USE_ALLOWED, false);

        match name.as_plain() {
            Some("at-root") => self.parse_at_root_rule(start),
            Some("content") => self.parse_content_rule(start),
            Some("debug") => self.parse_debug_rule(),
            Some("each") => self.parse_each_rule(child),
            Some("else") | Some("return") => self.parse_disallowed_at_rule(start),
            Some("error") => self.parse_error_rule(),
            Some("extend") => self.parse_extend_rule(start),
            Some("for") => self.parse_for_rule(child),
            Some("forward") => {
                self.flags_mut()
                    .set(ContextFlags::IS_USE_ALLOWED, was_use_allowed);
                // if (!root) {
                //     _disallowedAtRule();
                // }
                self.parse_forward_rule(start)
            }
            Some("function") => self.parse_function_rule(start),
            Some("if") => self.parse_if_rule(child),
            Some("import") => self.parse_import_rule(start),
            Some("include") => self.parse_include_rule(),
            Some("media") => self.parse_media_rule(start),
            Some("mixin") => self.parse_mixin_rule(start),
            // todo: support -moz-document
            // Some("-moz-document") => self.parse_moz_document_rule(name),
            Some("supports") => self.parse_supports_rule(),
            Some("use") => {
                self.flags_mut()
                    .set(ContextFlags::IS_USE_ALLOWED, was_use_allowed);
                // if (!root) {
                //     _disallowedAtRule();
                // }
                self.parse_use_rule(start)
            }
            Some("warn") => self.parse_warn_rule(),
            Some("while") => self.parse_while_rule(child),
            Some(..) | None => self.unknown_at_rule(name, start),
        }
    }

    fn parse_loud_comment(&mut self) -> SassResult<AstLoudComment> {
        let start = self.toks().cursor();
        self.expect_char('/')?;
        self.expect_char('*')?;

        let mut buffer = Interpolation::new_plain("/*".to_owned());

        while let Some(tok) = self.toks().peek() {
            match tok.kind {
                '#' => {
                    if matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })) {
                        buffer.add_interpolation(self.parse_single_interpolation()?);
                    } else {
                        self.toks_mut().next();
                        buffer.add_char(tok.kind);
                    }
                }
                '*' => {
                    self.toks_mut().next();
                    buffer.add_char(tok.kind);

                    if self.scan_char('/') {
                        buffer.add_char('/');

                        return Ok(AstLoudComment {
                            text: buffer,
                            span: self.toks_mut().span_from(start),
                        });
                    }
                }
                '\r' => {
                    self.toks_mut().next();
                    // todo: does \r even exist at this point? (removed by lexer)
                    if !self.toks_mut().next_char_is('\n') {
                        buffer.add_char('\n');
                    }
                }
                _ => {
                    buffer.add_char(tok.kind);
                    self.toks_mut().next();
                }
            }
        }

        Err(("expected more input.", self.toks().current_span()).into())
    }

    fn parse_silent_comment(&mut self) -> SassResult<AstStmt> {
        let start = self.toks().cursor();
        debug_assert!(self.next_matches("//"));
        self.toks_mut().next();
        self.toks_mut().next();

        let mut buffer = String::new();

        while let Some(tok) = self.toks_mut().next() {
            if tok.kind == '\n' {
                self.whitespace_without_comments();
                if self.next_matches("//") {
                    self.toks_mut().next();
                    self.toks_mut().next();
                    buffer.clear();
                    continue;
                }
                break;
            }

            buffer.push(tok.kind);
        }

        if self.is_plain_css() {
            return Err((
                "Silent comments aren't allowed in plain CSS.",
                self.toks_mut().span_from(start),
            )
                .into());
        }

        self.whitespace_without_comments();

        Ok(AstStmt::SilentComment(AstSilentComment {
            text: buffer,
            span: self.toks_mut().span_from(start),
        }))
    }
}
