//! Statements: style rules, declarations and the at-rules any context allows.

use super::*;

/// Statements: style rules, declarations and the at-rules any context allows.
pub(crate) trait StatementParser<'a>: StylesheetParser<'a> {
    fn plain_at_rule_name(&mut self) -> SassResult<String> {
        self.expect_char('@')?;
        let name = self.parse_identifier(false, false)?;
        self.whitespace()?;
        Ok(name)
    }

    fn with_children(
        &mut self,
        child: fn(&mut Self) -> SassResult<AstStmt>,
    ) -> SassResult<Spanned<Vec<AstStmt>>> {
        let start = self.toks().cursor();
        let children = self.parse_children(child)?;
        let span = self.toks_mut().span_from(start);
        self.whitespace_without_comments();
        Ok(Spanned {
            node: children,
            span,
        })
    }

    fn parse_at_root_query(&mut self) -> SassResult<Interpolation> {
        let mut buffer = Interpolation::new();
        self.expect_char('(')?;
        buffer.add_char('(');

        self.whitespace_with_newlines()?;

        buffer.add_expr(self.parse_expression_with_newlines(None)?);

        if self.scan_char(':') {
            self.whitespace_with_newlines()?;
            buffer.add_char(':');
            buffer.add_char(' ');
            buffer.add_expr(self.parse_expression_with_newlines(None)?);
        }

        self.expect_char(')')?;
        self.whitespace()?;
        buffer.add_char(')');

        Ok(buffer)
    }

    fn parse_at_root_rule(&mut self, start: usize) -> SassResult<AstStmt> {
        Ok(AstStmt::AtRootRule(if self.toks_mut().next_char_is('(') {
            let query_start = self.toks().cursor();
            let query = self.parse_at_root_query()?;
            let query_span = self.toks_mut().span_from(query_start);
            self.whitespace()?;
            let children = self.with_children(Self::parse_statement)?.node;

            AstAtRootRule {
                query: Some(Spanned {
                    node: query,
                    span: query_span,
                }),
                body: children,
                span: self.toks_mut().span_from(start),
            }
        } else if self.looking_at_children()? {
            let children = self.with_children(Self::parse_statement)?.node;
            AstAtRootRule {
                query: None,
                body: children,
                span: self.toks_mut().span_from(start),
            }
        } else {
            let child = self.parse_style_rule(None, None)?;
            AstAtRootRule {
                query: None,
                body: vec![child],
                span: self.toks_mut().span_from(start),
            }
        }))
    }

    fn parse_disallowed_at_rule(&mut self, start: usize) -> SassResult<AstStmt> {
        self.almost_any_value(false)?;
        Err((
            "This at-rule is not allowed here.",
            self.toks_mut().span_from(start),
        )
            .into())
    }

    fn parse_extend_rule(&mut self, start: usize) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        if !self.flags().in_style_rule()
            && !self.flags().in_mixin()
            && !self.flags().in_content_block()
        {
            return Err((
                "@extend may only be used within style rules.",
                self.toks_mut().span_from(start),
            )
                .into());
        }

        let value = self.almost_any_value(false)?;

        let is_optional = self.scan_char('!');

        if is_optional {
            self.expect_identifier("optional", false)?;
        }

        self.expect_statement_separator(Some("@extend rule"))?;

        Ok(AstStmt::Extend(AstExtendRule {
            value,
            is_optional,
            span: self.toks_mut().span_from(start),
        }))
    }

    fn parse_media_rule(&mut self, start: usize) -> SassResult<AstStmt> {
        let query_start = self.toks().cursor();
        let query = self.parse_media_query_list()?;
        let query_span = self.toks_mut().span_from(query_start);

        let body = self.with_children(Self::parse_statement)?.node;

        Ok(AstStmt::Media(AstMedia {
            query,
            query_span,
            body,
            span: self.toks_mut().span_from(start),
        }))
    }

    fn _parse_moz_document_rule(&mut self, _name: Interpolation) -> SassResult<AstStmt> {
        todo!("special cased @-moz-document not yet implemented")
    }

    fn unknown_at_rule(&mut self, name: Interpolation, start: usize) -> SassResult<AstStmt> {
        let was_in_unknown_at_rule = self.flags().in_unknown_at_rule();
        self.flags_mut().set(ContextFlags::IN_UNKNOWN_AT_RULE, true);

        let value: Option<Interpolation> =
            if !self.toks_mut().next_char_is('!') && !self.at_end_of_statement() {
                Some(self.almost_any_value(false)?)
            } else {
                None
            };

        let children = if self.looking_at_children()? {
            Some(self.with_children(Self::parse_statement)?.node)
        } else {
            self.expect_statement_separator(None)?;
            None
        };

        self.flags_mut()
            .set(ContextFlags::IN_UNKNOWN_AT_RULE, was_in_unknown_at_rule);

        Ok(AstStmt::UnknownAtRule(AstUnknownAtRule {
            name,
            value,
            body: children,
            span: self.toks_mut().span_from(start),
        }))
    }

    fn parse_statement(&mut self) -> SassResult<AstStmt> {
        match self.toks().peek() {
            Some(Token { kind: '@', .. }) => self.parse_at_rule(Self::parse_statement),
            Some(Token { kind: '+', .. }) => {
                if !self.is_indented() {
                    return self.parse_style_rule(None, None);
                }

                let start = self.toks().cursor();

                self.toks_mut().next();

                if !self.looking_at_identifier() {
                    self.toks_mut().set_cursor(start);
                    return self.parse_style_rule(None, None);
                }

                self.flags_mut().set(ContextFlags::IS_USE_ALLOWED, false);
                self.parse_include_rule()
            }
            Some(Token { kind: '=', .. }) => {
                if !self.is_indented() {
                    return self.parse_style_rule(None, None);
                }

                self.flags_mut().set(ContextFlags::IS_USE_ALLOWED, false);
                let start = self.toks().cursor();
                self.toks_mut().next();
                self.whitespace_with_newlines()?;
                self.parse_mixin_rule(start)
            }
            Some(Token { kind: '}', .. }) => {
                Err(("unmatched \"}\".", self.toks().current_span()).into())
            }
            _ => {
                if self.flags().in_style_rule()
                    || self.flags().in_unknown_at_rule()
                    || self.flags().in_mixin()
                    || self.flags().in_content_block()
                {
                    self.parse_declaration_or_style_rule()
                } else {
                    self.parse_variable_declaration_or_style_rule()
                }
            }
        }
    }
}

impl<'a, T: StylesheetParser<'a>> StatementParser<'a> for T {}
