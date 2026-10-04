use std::path::Path;

use codemap::Span;

use crate::{ContextFlags, Options, Token, ast::*, error::SassResult, lexer::Lexer};

use super::{BaseParser, IdentifierParser, StylesheetParser};

use super::{InterpolationParser, RawValueParser, VariableParser};

mod indentation;

pub(crate) struct SassParser<'a> {
    pub toks: Lexer,
    pub path: &'a Path,
    pub empty_span: Span,
    pub flags: ContextFlags,
    pub options: &'a Options<'a>,
    pub current_indentation: usize,
    pub next_indentation: Option<usize>,
    pub spaces: Option<bool>,
    pub next_indentation_end: Option<usize>,
}

impl<'a> BaseParser for SassParser<'a> {
    fn toks(&self) -> &Lexer {
        &self.toks
    }

    fn toks_mut(&mut self) -> &mut Lexer {
        &mut self.toks
    }

    fn whitespace_without_comments(&mut self) {
        while let Some(next) = self.toks.peek() {
            if next.kind != '\t' && next.kind != ' ' {
                break;
            }

            self.toks.next();
        }
    }
}

impl<'a> StylesheetParser<'a> for SassParser<'a> {
    fn is_plain_css(&self) -> bool {
        false
    }

    fn is_indented(&self) -> bool {
        true
    }

    fn path(&self) -> &'a Path {
        self.path
    }

    fn options(&self) -> &Options<'_> {
        self.options
    }

    fn flags(&self) -> &ContextFlags {
        &self.flags
    }

    fn flags_mut(&mut self) -> &mut ContextFlags {
        &mut self.flags
    }

    fn current_indentation(&self) -> usize {
        self.current_indentation
    }

    fn empty_span(&self) -> Span {
        self.empty_span
    }

    fn parse_style_rule_selector(&mut self) -> SassResult<Interpolation> {
        let mut buffer = Interpolation::new();

        loop {
            buffer.add_interpolation(self.almost_any_value(true)?);
            if buffer.trailing_string().trim_end().ends_with(',') && self.scan_char('\n') {
                buffer.add_char('\n');
            } else {
                break;
            }
        }

        Ok(buffer)
    }

    fn expect_statement_separator(&mut self, name: Option<&str>) -> SassResult<()> {
        let trailing_semicolon = self.try_trailing_semicolon()?;
        if !self.at_end_of_statement() {
            self.expect_newline(trailing_semicolon)?;
        }

        if self.peek_indentation()? <= self.current_indentation {
            return Ok(());
        }

        // todo: position: _nextIndentationEnd!.position
        let message = match name {
            Some(name) => format!("Nothing may be indented beneath a {name}."),
            None => "Nothing may be indented here.".to_owned(),
        };
        Err((message, self.toks.current_span()).into())
    }

    fn at_end_of_statement(&self) -> bool {
        matches!(self.toks.peek(), Some(Token { kind: '\n', .. }) | None)
    }

    fn looking_at_children(&mut self) -> SassResult<bool> {
        Ok(self.at_end_of_statement() && self.peek_indentation()? > self.current_indentation)
    }

    fn scan_else(&mut self, if_indentation: usize) -> SassResult<bool> {
        if self.peek_indentation()? != if_indentation {
            return Ok(false);
        }

        let start = self.toks.cursor();
        let start_indentation = self.current_indentation;
        let start_next_indentation = self.next_indentation;
        let start_next_indentation_end = self.next_indentation_end;

        self.read_indentation()?;
        if self.scan_char('@') && self.scan_identifier("else", false)? {
            return Ok(true);
        }

        self.toks.set_cursor(start);
        self.current_indentation = start_indentation;
        self.next_indentation = start_next_indentation;
        self.next_indentation_end = start_next_indentation_end;
        Ok(false)
    }

    fn parse_children(
        &mut self,
        child: fn(&mut Self) -> SassResult<AstStmt>,
    ) -> SassResult<Vec<AstStmt>> {
        let mut children = Vec::new();
        self.while_indented_lower(|parser| {
            if let Some(parsed_child) = parser.parse_child(|parser| Ok(Some(child(parser)?)))? {
                children.push(parsed_child);
            }

            Ok(())
        })?;

        Ok(children)
    }

    fn parse_statements(
        &mut self,
        statement: fn(&mut Self) -> SassResult<Option<AstStmt>>,
    ) -> SassResult<Vec<AstStmt>> {
        if self.toks.next_char_is(' ') || self.toks.next_char_is('\t') {
            return Err((
                "Indenting at the beginning of the document is illegal.",
                self.toks.current_span(),
            )
                .into());
        }

        let mut statements = Vec::new();

        while self.toks.peek().is_some() {
            if let Some(child) = self.parse_child(statement)? {
                statements.push(child);
            }

            let indentation = self.read_indentation()?;
            // dart-sass asserts this (an assertion its releases leave out).
            debug_assert_eq!(indentation, 0);
        }

        Ok(statements)
    }

    fn parse_silent_comment(&mut self) -> SassResult<AstStmt> {
        let start = self.toks.cursor();
        self.expect_char('/')?;
        self.expect_char('/')?;

        let mut buffer = String::new();

        let parent_indentation = self.current_indentation;

        'outer: loop {
            let comment_prefix = if self.scan_char('/') { "///" } else { "//" };

            loop {
                buffer.push_str(comment_prefix);
                //     buffer.write(commentPrefix);

                // Skip the initial characters because we're already writing the
                // slashes.
                for _ in comment_prefix.len()..(self.current_indentation - parent_indentation) {
                    buffer.push(' ');
                }

                while self.toks.peek().is_some() && !self.toks.next_char_is('\n') {
                    buffer.push(self.toks.next().unwrap().kind);
                }

                buffer.push('\n');

                if self.peek_indentation()? < parent_indentation {
                    break 'outer;
                }

                if self.peek_indentation()? == parent_indentation {
                    // Look ahead to the next line to see if it starts another comment.
                    if matches!(
                        self.toks.peek_n(1 + parent_indentation),
                        Some(Token { kind: '/', .. })
                    ) && matches!(
                        self.toks.peek_n(2 + parent_indentation),
                        Some(Token { kind: '/', .. })
                    ) {
                        self.read_indentation()?;
                    }
                    break;
                }

                self.read_indentation()?;
            }

            if !self.scan("//") {
                break;
            }
        }

        Ok(AstStmt::SilentComment(AstSilentComment {
            text: buffer,
            span: self.toks.span_from(start),
        }))
    }

    fn parse_loud_comment(&mut self) -> SassResult<AstLoudComment> {
        let start = self.toks.cursor();
        self.expect_char('/')?;
        self.expect_char('*')?;

        let mut first = true;

        let mut buffer = Interpolation::new_plain("/*".to_owned());
        let parent_indentation = self.current_indentation;

        loop {
            if first {
                let beginning_of_comment = self.toks.cursor();

                self.spaces();

                if self.toks.next_char_is('\n') {
                    self.read_indentation()?;
                    buffer.add_char(' ');
                } else {
                    buffer.add_string(self.toks.raw_text(beginning_of_comment));
                }
            } else {
                buffer.add_string("\n * ".to_owned());
            }

            first = false;

            for _ in 3..(self.current_indentation - parent_indentation) {
                buffer.add_char(' ');
            }

            while self.toks.peek().is_some() {
                match self.toks.peek() {
                    Some(Token {
                        kind: '\n' | '\r', ..
                    }) => break,
                    Some(Token { kind: '#', .. }) => {
                        if matches!(self.toks.peek_n(1), Some(Token { kind: '{', .. })) {
                            buffer.add_interpolation(self.parse_single_interpolation()?);
                        } else {
                            buffer.add_char('#');
                            self.toks.next();
                        }
                    }
                    Some(Token { kind, .. }) => {
                        buffer.add_char(kind);
                        self.toks.next();
                    }
                    None => todo!(),
                }
            }

            if self.peek_indentation()? <= parent_indentation {
                break;
            }

            // Preserve empty lines.
            while self.looking_at_double_newline() {
                self.expect_newline(false)?;
                buffer.add_char('\n');
                buffer.add_char(' ');
                buffer.add_char('*');
            }

            self.read_indentation()?;
        }

        if !buffer.trailing_string().trim_end().ends_with("*/") {
            buffer.add_string(" */".to_owned());
        }

        Ok(AstLoudComment {
            text: buffer,
            span: self.toks.span_from(start),
        })
    }
}
