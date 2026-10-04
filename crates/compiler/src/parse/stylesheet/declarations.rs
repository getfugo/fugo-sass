//! Declarations and style rules.

use super::*;

/// Declarations and style rules.
pub(crate) trait DeclarationParser<'a>: StylesheetParser<'a> {
    fn parse_declaration_or_style_rule(&mut self) -> SassResult<AstStmt> {
        let start = self.toks().cursor();

        // The indented syntax allows a single backslash to distinguish a style rule
        // from old-style property syntax. We don't support old property syntax, but
        // we do support the backslash because it's easy to do.
        if self.is_indented() && self.scan_char('\\') {
            return self.parse_style_rule(None, None);
        };

        match self.parse_declaration_or_buffer()? {
            DeclarationOrBuffer::Stmt(s) => Ok(s),
            DeclarationOrBuffer::Buffer(existing_buffer) => {
                self.parse_style_rule(Some(existing_buffer), Some(start))
            }
        }
    }

    fn parse_property_or_variable_declaration(
        &mut self,
        // default=true
        parse_custom_properties: bool,
    ) -> SassResult<AstStmt> {
        let start = self.toks().cursor();

        let name = if matches!(
            self.toks().peek(),
            Some(Token {
                kind: ':' | '*' | '.',
                ..
            })
        ) || (matches!(self.toks().peek(), Some(Token { kind: '#', .. }))
            && !matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })))
        {
            // Allow the "*prop: val", ":prop: val", "#prop: val", and ".prop: val"
            // hacks.
            let mut name_buffer = Interpolation::new();
            name_buffer.add_char(self.toks_mut().next().unwrap().kind);
            name_buffer.add_string(self.raw_text(Self::whitespace));
            name_buffer.add_interpolation(self.parse_interpolated_identifier()?);
            name_buffer
        } else if !self.is_plain_css() {
            match self.parse_variable_declaration_or_interpolation()? {
                VariableDeclOrInterpolation::Interpolation(interpolation) => interpolation,
                VariableDeclOrInterpolation::VariableDecl(decl) => {
                    return Ok(AstStmt::VariableDecl(decl));
                }
            }
        } else {
            self.parse_interpolated_identifier()?
        };

        self.whitespace()?;
        self.expect_char(':')?;

        if parse_custom_properties && name.initial_plain().starts_with("--") {
            let interpolation =
                self.parse_interpolated_declaration_value(false, false, true, false)?;
            let value_span = self.toks_mut().span_from(start);
            let value = AstExpr::String(StringExpr(interpolation, QuoteKind::None), value_span)
                .span(value_span);
            self.expect_statement_separator(Some("custom property"))?;
            return Ok(AstStmt::Style(AstStyle {
                name,
                value: Some(value),
                body: Vec::new(),
                span: value_span,
            }));
        }

        self.whitespace()?;

        if self.looking_at_children()? {
            if self.is_plain_css() {
                return Err((
                    "Nested declarations aren't allowed in plain CSS.",
                    self.toks().current_span(),
                )
                    .into());
            }

            if name.initial_plain().starts_with("--") {
                return Err((
                    "Declarations whose names begin with \"--\" may not be nested",
                    self.toks_mut().span_from(start),
                )
                    .into());
            }

            let children = self.with_children(Self::parse_declaration_child)?.node;

            return Ok(AstStmt::Style(AstStyle {
                name,
                value: None,
                body: children,
                span: self.toks_mut().span_from(start),
            }));
        }

        let value = self.parse_expression(None, None, None)?;
        if self.looking_at_children()? {
            if self.is_plain_css() {
                return Err((
                    "Nested declarations aren't allowed in plain CSS.",
                    self.toks().current_span(),
                )
                    .into());
            }

            if name.initial_plain().starts_with("--") && !matches!(value.node, AstExpr::String(..))
            {
                return Err((
                    "Declarations whose names begin with \"--\" may not be nested",
                    self.toks_mut().span_from(start),
                )
                    .into());
            }

            let children = self.with_children(Self::parse_declaration_child)?.node;

            Ok(AstStmt::Style(AstStyle {
                name,
                value: Some(value),
                body: children,
                span: self.toks_mut().span_from(start),
            }))
        } else {
            self.expect_statement_separator(None)?;
            Ok(AstStmt::Style(AstStyle {
                name,
                value: Some(value),
                body: Vec::new(),
                span: self.toks_mut().span_from(start),
            }))
        }
    }

    fn parse_declaration_or_buffer(&mut self) -> SassResult<DeclarationOrBuffer> {
        let start = self.toks().cursor();
        let mut name_buffer = Interpolation::new();

        // Allow the "*prop: val", ":prop: val", "#prop: val", and ".prop: val"
        // hacks.
        let first = self.toks().peek();
        let mut starts_with_punctuation = false;

        if matches!(
            first,
            Some(Token {
                kind: ':' | '*' | '.',
                ..
            })
        ) || (matches!(first, Some(Token { kind: '#', .. }))
            && !matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })))
        {
            starts_with_punctuation = true;
            name_buffer.add_char(self.toks_mut().next().unwrap().kind);
            name_buffer.add_string(self.raw_text(Self::whitespace));
        }

        if !self.looking_at_interpolated_identifier() {
            return Ok(DeclarationOrBuffer::Buffer(name_buffer));
        }

        let variable_or_interpolation = if starts_with_punctuation {
            VariableDeclOrInterpolation::Interpolation(self.parse_interpolated_identifier()?)
        } else {
            self.parse_variable_declaration_or_interpolation()?
        };

        match variable_or_interpolation {
            VariableDeclOrInterpolation::Interpolation(int) => name_buffer.add_interpolation(int),
            VariableDeclOrInterpolation::VariableDecl(v) => {
                return Ok(DeclarationOrBuffer::Stmt(AstStmt::VariableDecl(v)));
            }
        }

        self.flags_mut().set(ContextFlags::IS_USE_ALLOWED, false);

        if self.next_matches("/*") {
            name_buffer.add_string(self.fallible_raw_text(Self::skip_loud_comment)?);
        }

        let mut mid_buffer = String::new();
        mid_buffer.push_str(&self.raw_text(Self::whitespace));

        if !self.scan_char(':') {
            if !mid_buffer.is_empty() {
                name_buffer.add_char(' ');
            }
            return Ok(DeclarationOrBuffer::Buffer(name_buffer));
        }
        mid_buffer.push(':');

        // Parse custom properties as declarations no matter what.
        if name_buffer.initial_plain().starts_with("--") {
            let value_start = self.toks().cursor();
            let value = self.parse_interpolated_declaration_value(false, false, true, false)?;
            let value_span = self.toks_mut().span_from(value_start);
            self.expect_statement_separator(Some("custom property"))?;
            return Ok(DeclarationOrBuffer::Stmt(AstStmt::Style(AstStyle {
                name: name_buffer,
                value: Some(
                    AstExpr::String(StringExpr(value, QuoteKind::None), value_span)
                        .span(value_span),
                ),
                span: self.toks_mut().span_from(start),
                body: Vec::new(),
            })));
        }

        if self.scan_char(':') {
            name_buffer.add_string(mid_buffer);
            name_buffer.add_char(':');
            return Ok(DeclarationOrBuffer::Buffer(name_buffer));
        } else if self.is_indented() && self.looking_at_interpolated_identifier() {
            // In the indented syntax, `foo:bar` is always considered a selector
            // rather than a property.
            name_buffer.add_string(mid_buffer);
            return Ok(DeclarationOrBuffer::Buffer(name_buffer));
        }

        let post_colon_whitespace = self.raw_text(Self::whitespace);
        if self.looking_at_children()? {
            if self.is_plain_css() {
                return Err((
                    "Nested declarations aren't allowed in plain CSS.",
                    self.toks().current_span(),
                )
                    .into());
            }
            let body = self.with_children(Self::parse_declaration_child)?.node;
            return Ok(DeclarationOrBuffer::Stmt(AstStmt::Style(AstStyle {
                name: name_buffer,
                value: None,
                span: self.toks_mut().span_from(start),
                body,
            })));
        }

        mid_buffer.push_str(&post_colon_whitespace);
        let could_be_selector =
            post_colon_whitespace.is_empty() && self.looking_at_interpolated_identifier();

        let before_decl = self.toks().cursor();

        let mut calculate_value = || {
            let value = self.parse_expression(None, None, None)?;

            if self.looking_at_children()? {
                if could_be_selector {
                    self.expect_statement_separator(None)?;
                }
            } else if !self.at_end_of_statement() {
                self.expect_statement_separator(None)?;
            }

            Ok(value)
        };

        let value = match calculate_value() {
            Ok(v) => v,
            Err(e) => {
                if !could_be_selector {
                    return Err(e);
                }

                self.toks_mut().set_cursor(before_decl);
                let additional = self.almost_any_value(false)?;
                if !self.is_indented() && self.toks_mut().next_char_is(';') {
                    return Err(e);
                }

                name_buffer.add_string(mid_buffer);
                name_buffer.add_interpolation(additional);
                return Ok(DeclarationOrBuffer::Buffer(name_buffer));
            }
        };

        if self.looking_at_children()? {
            if self.is_plain_css() {
                return Err((
                    "Nested declarations aren't allowed in plain CSS.",
                    self.toks().current_span(),
                )
                    .into());
            }
            let body = self.with_children(Self::parse_declaration_child)?.node;
            Ok(DeclarationOrBuffer::Stmt(AstStmt::Style(AstStyle {
                name: name_buffer,
                value: Some(value),
                span: self.toks_mut().span_from(start),
                body,
            })))
        } else {
            self.expect_statement_separator(None)?;
            Ok(DeclarationOrBuffer::Stmt(AstStmt::Style(AstStyle {
                name: name_buffer,
                value: Some(value),
                span: self.toks_mut().span_from(start),
                body: Vec::new(),
            })))
        }
    }

    fn parse_declaration_child(&mut self) -> SassResult<AstStmt> {
        let start = self.toks().cursor();

        if self.toks_mut().next_char_is('@') {
            self.parse_declaration_at_rule(start)
        } else {
            self.parse_property_or_variable_declaration(false)
        }
    }

    fn parse_plain_at_rule_name(&mut self) -> SassResult<String> {
        self.expect_char('@')?;
        let name = self.parse_identifier(false, false)?;
        self.whitespace()?;
        Ok(name)
    }

    fn parse_declaration_at_rule(&mut self, start: usize) -> SassResult<AstStmt> {
        let name = self.parse_plain_at_rule_name()?;

        match name.as_str() {
            "content" => self.parse_content_rule(start),
            "debug" => self.parse_debug_rule(),
            "each" => self.parse_each_rule(Self::parse_declaration_child),
            "else" => self.parse_disallowed_at_rule(start),
            "error" => self.parse_error_rule(),
            "for" => self.parse_for_rule(Self::parse_declaration_child),
            "if" => self.parse_if_rule(Self::parse_declaration_child),
            "include" => self.parse_include_rule(),
            "warn" => self.parse_warn_rule(),
            "while" => self.parse_while_rule(Self::parse_declaration_child),
            _ => self.parse_disallowed_at_rule(start),
        }
    }

    fn parse_variable_declaration_or_style_rule(&mut self) -> SassResult<AstStmt> {
        let start = self.toks().cursor();

        if self.is_plain_css() {
            return self.parse_style_rule(None, None);
        }

        // The indented syntax allows a single backslash to distinguish a style rule
        // from old-style property syntax. We don't support old property syntax, but
        // we do support the backslash because it's easy to do.
        if self.is_indented() && self.scan_char('\\') {
            return self.parse_style_rule(None, None);
        };

        if !self.looking_at_identifier() {
            return self.parse_style_rule(None, None);
        }

        match self.parse_variable_declaration_or_interpolation()? {
            VariableDeclOrInterpolation::VariableDecl(var) => Ok(AstStmt::VariableDecl(var)),
            VariableDeclOrInterpolation::Interpolation(int) => {
                self.parse_style_rule(Some(int), Some(start))
            }
        }
    }

    fn parse_style_rule(
        &mut self,
        existing_buffer: Option<Interpolation>,
        start: Option<usize>,
    ) -> SassResult<AstStmt> {
        let start = start.unwrap_or_else(|| self.toks().cursor());

        self.flags_mut().set(ContextFlags::IS_USE_ALLOWED, false);
        let mut interpolation = self.parse_style_rule_selector()?;

        if let Some(mut existing_buffer) = existing_buffer {
            existing_buffer.add_interpolation(interpolation);
            interpolation = existing_buffer;
        }

        if interpolation.contents.is_empty() {
            return Err(("expected \"}\".", self.toks().current_span()).into());
        }

        let was_in_style_rule = self.flags().in_style_rule();
        *self.flags_mut() |= ContextFlags::IN_STYLE_RULE;

        let selector_span = self.toks_mut().span_from(start);

        let children = self.with_children(Self::parse_statement)?;

        self.flags_mut()
            .set(ContextFlags::IN_STYLE_RULE, was_in_style_rule);

        let span = selector_span.merge(children.span);

        Ok(AstStmt::RuleSet(AstRuleSet {
            selector: interpolation,
            body: children.node,
            selector_span,
            span,
        }))
    }
}

impl<'a, T: StylesheetParser<'a>> DeclarationParser<'a> for T {}
