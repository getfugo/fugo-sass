//! The CSS `if()` function.

use super::*;

/// The CSS `if()` function.
pub(crate) trait CssIfParser<'a>: StylesheetParser<'a> {
    /// Parses a CSS `if()` expression, after its name (which starts at `start`).
    fn parse_if_expression(&mut self, start: usize) -> SassResult<Spanned<AstExpr>> {
        self.expect_char('(')?;
        self.whitespace_with_newlines()?;
        let mut branches = Vec::new();
        while !self.toks().next_char_is(')') {
            let condition = if self.scan_identifier("else", false)? {
                None
            } else {
                Some(self.parse_if_condition()?)
            };
            self.whitespace_with_newlines()?;
            self.expect_char(':')?;
            self.whitespace_with_newlines()?;
            branches.push((condition, self.parse_expression_with_newlines(None)?));
            self.whitespace_with_newlines()?;
            if !self.scan_char(';') {
                break;
            }
            self.whitespace_with_newlines()?;
        }
        self.expect_char(')')?;

        let span = self.toks_mut().span_from(start);
        if branches.is_empty() {
            return Err(("Expected identifier.", span).into());
        }
        Ok(AstExpr::If(Arc::new(IfExpr { branches, span })).span(span))
    }

    /// The error for a function-style `and`, `or` or `not` in an `if()` condition.
    fn if_whitespace_error<T>(&mut self, name: &str) -> SassResult<T> {
        Err((
            format!("Whitespace is required between \"{name}\" and \"(\""),
            self.toks().current_span(),
        )
            .into())
    }

    fn parse_if_condition(&mut self) -> SassResult<IfCondition> {
        let start = self.toks().cursor();
        if self.scan_identifier("not", false)? {
            if self.toks().next_char_is('(') {
                return self.if_whitespace_error("not");
            }
            self.whitespace_with_newlines()?;
            let group = self.parse_if_group()?;
            return Ok(IfCondition::Negation(
                Box::new(group),
                self.toks_mut().span_from(start),
            ));
        }

        let mut groups = vec![self.parse_if_group()?];
        let mut op = None;

        fn combine(mut groups: Vec<IfCondition>, op: Option<IfConditionOp>) -> IfCondition {
            match op {
                Some(op) if groups.len() > 1 => IfCondition::Operation(groups, op),
                _ => groups.pop().unwrap(),
            }
        }

        self.whitespace_with_newlines()?;
        loop {
            if op != Some(IfConditionOp::Or) && self.scan_identifier("and", false)? {
                if self.toks().next_char_is('(') {
                    return self.if_whitespace_error("and");
                }
                self.whitespace_with_newlines()?;
                op.get_or_insert(IfConditionOp::And);
                groups.push(self.parse_if_group()?);
            } else if op != Some(IfConditionOp::And) && self.scan_identifier("or", false)? {
                if self.toks().next_char_is('(') {
                    // dart-sass names "and" here too.
                    return self.if_whitespace_error("and");
                }
                self.whitespace_with_newlines()?;
                op.get_or_insert(IfConditionOp::Or);
                groups.push(self.parse_if_group()?);
            } else if !matches!(
                self.toks().peek(),
                None | Some(Token {
                    kind: ')' | ':',
                    ..
                })
            ) && groups.last().unwrap().is_arbitrary_substitution()
            {
                let next = self.parse_if_group()?;
                return self.parse_if_condition_raw(combine(groups, op), next);
            } else if let Some(substitution) = self.try_parse_arbitrary_substitution()? {
                return self.parse_if_condition_raw(combine(groups, op), substitution);
            } else {
                break;
            }

            self.whitespace_with_newlines()?;
        }

        Ok(combine(groups, op))
    }

    /// Parses the rest of what would have been an `IfCondition::Operation` as raw text, after
    /// `next` was parsed between two of its groups. `preceding` (or its last group) or `next` is an
    /// arbitrary substitution.
    fn parse_if_condition_raw(
        &mut self,
        preceding: IfCondition,
        next: IfCondition,
    ) -> SassResult<IfCondition> {
        let substitution = match &preceding {
            _ if preceding.is_arbitrary_substitution() => preceding.span(),
            IfCondition::Operation(expressions, _)
                if expressions.last().unwrap().is_arbitrary_substitution() =>
            {
                expressions.last().unwrap().span()
            }
            _ => next.span(),
        };

        let mut buffer = preceding.to_interpolation(substitution)?;
        buffer.add_char(' ');
        buffer.add_interpolation(next.to_interpolation(substitution)?);

        let mut last_group = next;
        let mut op = match preceding {
            IfCondition::Operation(_, op) => Some(op),
            _ => None,
        };

        self.whitespace_with_newlines()?;
        loop {
            if op != Some(IfConditionOp::Or) && self.scan_identifier("and", false)? {
                if self.toks().next_char_is('(') {
                    return self.if_whitespace_error("and");
                }
                self.whitespace_with_newlines()?;
                op.get_or_insert(IfConditionOp::And);
                // dart-sass doesn't update the last group after an `and`.
                let group = self.parse_if_group()?;
                buffer.add_string(" and ".to_owned());
                buffer.add_interpolation(group.to_interpolation(substitution)?);
            } else if op != Some(IfConditionOp::And) && self.scan_identifier("or", false)? {
                if self.toks().next_char_is('(') {
                    return self.if_whitespace_error("or");
                }
                self.whitespace_with_newlines()?;
                op.get_or_insert(IfConditionOp::Or);
                last_group = self.parse_if_group()?;
                self.whitespace_with_newlines()?;
                buffer.add_string(" or ".to_owned());
                buffer.add_interpolation(last_group.to_interpolation(substitution)?);
            } else if !matches!(
                self.toks().peek(),
                None | Some(Token {
                    kind: ')' | ':',
                    ..
                })
            ) && last_group.is_arbitrary_substitution()
            {
                last_group = self.parse_if_group()?;
                buffer.add_char(' ');
                buffer.add_interpolation(last_group.to_interpolation(substitution)?);
            } else if let Some(next) = self.try_parse_arbitrary_substitution()? {
                last_group = next;
                buffer.add_char(' ');
                buffer.add_interpolation(last_group.to_interpolation(substitution)?);
            } else {
                break;
            }

            self.whitespace_with_newlines()?;
        }

        let span = preceding.span().merge(self.toks().current_span());
        Ok(IfCondition::Raw(buffer, span))
    }

    /// Parses a group of an `if()` condition: a parenthesized condition, `sass()`, an
    /// interpolation, or a function.
    fn parse_if_group(&mut self) -> SassResult<IfCondition> {
        let start = self.toks().cursor();
        if self.toks().next_char_is('(') {
            self.expect_char('(')?;
            self.whitespace_with_newlines()?;
            let expression = self.parse_if_condition()?;
            self.whitespace_with_newlines()?;
            self.expect_char(')')?;
            return Ok(IfCondition::Parenthesized(
                Box::new(expression),
                self.toks_mut().span_from(start),
            ));
        }

        if self.scan_identifier("sass", true)? {
            self.expect_char('(')?;
            self.whitespace_with_newlines()?;
            let expression = self.parse_expression(None, None, None)?;
            self.whitespace_with_newlines()?;
            self.expect_char(')')?;
            let span = self.toks_mut().span_from(start);
            if self.is_plain_css() {
                return Err(("sass() conditions aren't allowed in plain CSS", span).into());
            }
            return Ok(IfCondition::Sass(expression, span));
        }

        let identifier = self.parse_interpolated_identifier()?;
        if !self.toks().next_char_is('(') {
            if let [InterpolationPart::Expr(..)] = identifier.contents.as_slice() {
                let span = self.toks_mut().span_from(start);
                return Ok(IfCondition::Raw(identifier, span));
            }
        } else if let Some(plain) = identifier.as_plain()
            && matches!(plain.to_ascii_lowercase().as_str(), "and" | "or" | "not")
        {
            let plain = plain.to_owned();
            return self.if_whitespace_error(&plain);
        }

        self.expect_char('(')?;
        self.whitespace_with_newlines()?;
        let arguments = self.parse_interpolated_declaration_value(true, true, true, true)?;
        self.whitespace_with_newlines()?;
        self.expect_char(')')?;
        Ok(IfCondition::Function {
            name: identifier,
            arguments,
            span: self.toks_mut().span_from(start),
        })
    }

    /// Parses an arbitrary substitution (an interpolation, or an `if()`, `var()`, `attr()` or
    /// custom function), if there's one.
    fn try_parse_arbitrary_substitution(&mut self) -> SassResult<Option<IfCondition>> {
        let start = self.toks().cursor();
        if self.toks().next_char_is('#') {
            let interpolation = self.parse_single_interpolation()?;
            let span = self.toks_mut().span_from(start);
            return Ok(Some(IfCondition::Raw(interpolation, span)));
        }

        let name = if self.scan_identifier("if", false)? {
            Interpolation::new_plain("if".to_owned())
        } else if self.scan_identifier("var", false)? {
            Interpolation::new_plain("var".to_owned())
        } else if self.scan_identifier("attr", false)? {
            Interpolation::new_plain("attr".to_owned())
        } else if self.next_matches("--") {
            self.parse_interpolated_identifier()?
        } else {
            return Ok(None);
        };

        if !self.scan_char('(') {
            self.toks_mut().set_cursor(start);
            return Ok(None);
        }

        let arguments = self.parse_interpolated_declaration_value(true, true, true, true)?;
        self.expect_char(')')?;
        Ok(Some(IfCondition::Function {
            name,
            arguments,
            span: self.toks_mut().span_from(start),
        }))
    }
}

impl<'a, T: StylesheetParser<'a>> CssIfParser<'a> for T {}
