//! `@import` and URLs.

use super::*;

/// `@import` and URLs.
pub(crate) trait ImportParser<'a>: StylesheetParser<'a> {
    fn try_parse_import_supports_function(&mut self) -> SassResult<Option<AstSupportsCondition>> {
        if !self.looking_at_interpolated_identifier() {
            return Ok(None);
        }

        let start = self.toks().cursor();
        let name = self.parse_interpolated_identifier()?;
        debug_assert!(name.as_plain() != Some("not"));

        if !self.scan_char('(') {
            self.toks_mut().set_cursor(start);
            return Ok(None);
        }

        let value = self.parse_interpolated_declaration_value(true, true, true, true)?;
        self.expect_char(')')?;

        Ok(Some(AstSupportsCondition::Function { name, args: value }))
    }

    fn parse_import_supports_query(&mut self) -> SassResult<AstSupportsCondition> {
        self.whitespace_with_newlines()?;
        Ok(if self.scan_identifier("not", false)? {
            self.whitespace_with_newlines()?;
            AstSupportsCondition::Negation(Box::new(self.supports_condition_in_parens()?))
        } else if self.toks_mut().next_char_is('(') {
            self.parse_supports_condition(true)?
        } else {
            match self.try_parse_import_supports_function()? {
                Some(function) => function,
                None => {
                    let start = self.toks().cursor();
                    let name = self.parse_expression_with_newlines(None)?;
                    self.expect_char(':')?;
                    self.supports_declaration_value(name.node, start)?
                }
            }
        })
    }

    fn try_import_modifiers(&mut self) -> SassResult<Option<Interpolation>> {
        // Exit before allocating anything if we're not looking at any modifiers, as
        // is the most common case.
        if !self.looking_at_interpolated_identifier() && !self.toks_mut().next_char_is('(') {
            return Ok(None);
        }

        let mut buffer = Interpolation::new();

        loop {
            if self.looking_at_interpolated_identifier() {
                if !buffer.is_empty() {
                    buffer.add_char(' ');
                }

                let identifier = self.parse_interpolated_identifier()?;
                let name = identifier.as_plain().map(str::to_ascii_lowercase);
                buffer.add_interpolation(identifier);

                if name.as_deref() != Some("and") && self.scan_char('(') {
                    if name.as_deref() == Some("supports") {
                        let query = self.parse_import_supports_query()?;
                        let is_declaration =
                            matches!(query, AstSupportsCondition::Declaration { .. });

                        if !is_declaration {
                            buffer.add_char('(');
                        }

                        buffer.add_expr(AstExpr::Supports(Arc::new(query)).span(self.empty_span()));

                        if !is_declaration {
                            buffer.add_char(')');
                        }
                    } else {
                        buffer.add_char('(');
                        buffer.add_interpolation(
                            self.parse_interpolated_declaration_value(true, true, true, true)?,
                        );
                        buffer.add_char(')');
                    }

                    self.expect_char(')')?;
                    self.whitespace()?;
                } else {
                    self.whitespace()?;
                    if self.scan_char(',') {
                        buffer.add_char(',');
                        buffer.add_char(' ');
                        buffer.add_interpolation(self.parse_media_query_list()?);
                        return Ok(Some(buffer));
                    }
                }
            } else if self.toks_mut().next_char_is('(') {
                if !buffer.is_empty() {
                    buffer.add_char(' ');
                }

                buffer.add_interpolation(self.parse_media_query_list()?);
                return Ok(Some(buffer));
            } else {
                return Ok(Some(buffer));
            }
        }
    }

    fn try_url_contents(&mut self, name: Option<&str>) -> SassResult<Option<Interpolation>> {
        let start = self.toks().cursor();
        if !self.scan_char('(') {
            return Ok(None);
        }
        self.whitespace_without_comments_with_newlines();

        // Match Ruby Sass's behavior: parse a raw URL() if possible, and if not
        // backtrack and re-parse as a function expression.
        let mut buffer = Interpolation::new();
        buffer.add_string(name.unwrap_or("url").to_owned());
        buffer.add_char('(');

        while let Some(next) = self.toks().peek() {
            match next.kind {
                '\\' => buffer.add_string(self.parse_escape(false)?),
                '!' | '%' | '&' | '*'..='~' | '\u{80}'..=char::MAX => {
                    self.toks_mut().next();
                    buffer.add_char(next.kind);
                }
                '#' => {
                    if matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })) {
                        let interpolation = self.parse_single_interpolation()?;
                        buffer.add_interpolation(interpolation);
                    } else {
                        self.toks_mut().next();
                        buffer.add_char(next.kind);
                    }
                }
                ')' => {
                    self.toks_mut().next();
                    buffer.add_char(next.kind);
                    return Ok(Some(buffer));
                }
                ' ' | '\t' | '\n' | '\r' => {
                    self.whitespace_without_comments_with_newlines();
                    if !self.toks_mut().next_char_is(')') {
                        break;
                    }
                }
                _ => break,
            }
        }

        self.toks_mut().set_cursor(start);

        Ok(None)
    }

    fn parse_dynamic_url(&mut self) -> SassResult<AstExpr> {
        let start = self.toks().cursor();
        self.expect_identifier("url", false)?;

        Ok(match self.try_url_contents(None)? {
            Some(contents) => AstExpr::String(
                StringExpr(contents, QuoteKind::None),
                self.toks_mut().span_from(start),
            ),
            None => AstExpr::InterpolatedFunction(Arc::new(InterpolatedFunction {
                name: Interpolation::new_plain("url".to_owned()),
                arguments: self.parse_argument_invocation(false, false)?,
                span: self.toks_mut().span_from(start),
            })),
        })
    }

    fn parse_import_argument(&mut self, start: usize) -> SassResult<AstImport> {
        if self.toks_mut().next_char_is('u') || self.toks_mut().next_char_is('U') {
            let url = self.parse_dynamic_url()?;
            self.whitespace()?;
            let modifiers = self.try_import_modifiers()?;
            return Ok(AstImport::Plain(AstPlainCssImport {
                url: Interpolation::new_with_expr(url.span(self.toks_mut().span_from(start))),
                modifiers,
                span: self.toks_mut().span_from(start),
            }));
        }

        let start = self.toks().cursor();
        let url = self.parse_string()?;
        let raw_url = self.toks().raw_text(start);
        self.whitespace()?;
        let modifiers = self.try_import_modifiers()?;

        let span = self.toks_mut().span_from(start);

        if is_plain_css_import(&url) || modifiers.is_some() {
            Ok(AstImport::Plain(AstPlainCssImport {
                url: Interpolation::new_plain(raw_url),
                modifiers,
                span,
            }))
        } else {
            // todo: try parseImportUrl
            Ok(AstImport::Sass(AstSassImport { url, span }))
        }
    }

    fn parse_import_rule(&mut self, start: usize) -> SassResult<AstStmt> {
        let mut imports = Vec::new();

        loop {
            self.whitespace()?;
            let argument = self.parse_import_argument(self.toks().cursor())?;

            // todo: _inControlDirective
            if (self.flags().in_control_flow() || self.flags().in_mixin()) && argument.is_dynamic()
            {
                self.parse_disallowed_at_rule(start)?;
            }

            imports.push(argument);
            self.whitespace()?;

            if !self.scan_char(',') {
                break;
            }
        }

        self.expect_statement_separator(Some("@import rule"))?;

        Ok(AstStmt::ImportRule(AstImportRule { imports }))
    }
}

impl<'a, T: StylesheetParser<'a>> ImportParser<'a> for T {}
