//! `@supports` conditions.

use super::*;

/// `@supports` conditions.
pub(crate) trait SupportsParser<'a>: StylesheetParser<'a> {
    fn try_supports_operation(
        &mut self,
        interpolation: &Interpolation,
        _start: usize,
    ) -> SassResult<Option<AstSupportsCondition>> {
        if interpolation.contents.len() != 1 {
            return Ok(None);
        }

        let expression = match interpolation.contents.first() {
            Some(InterpolationPart::Expr(e)) => e,
            Some(InterpolationPart::String(..)) => return Ok(None),
            None => unreachable!(),
        };

        let before_whitespace = self.toks().cursor();
        self.whitespace_with_newlines()?;

        let mut operation: Option<AstSupportsCondition> = None;
        let mut operator: Option<String> = None;

        while self.looking_at_identifier() {
            if let Some(operator) = &operator {
                self.expect_identifier(operator, false)?;
            } else if self.scan_identifier("and", false)? {
                operator = Some("and".to_owned());
            } else if self.scan_identifier("or", false)? {
                operator = Some("or".to_owned());
            } else {
                self.toks_mut().set_cursor(before_whitespace);
                return Ok(None);
            }

            self.whitespace_with_newlines()?;

            let right = self.supports_condition_in_parens()?;
            operation = Some(AstSupportsCondition::Operation {
                left: Box::new(operation.unwrap_or_else(|| {
                    AstSupportsCondition::Interpolation(expression.clone().node)
                })),
                operator: operator.clone(),
                right: Box::new(right),
            });
            self.whitespace_with_newlines()?;
        }

        Ok(operation)
    }

    fn supports_declaration_value(
        &mut self,
        name: AstExpr,
        start: usize,
    ) -> SassResult<AstSupportsCondition> {
        let value = match &name {
            AstExpr::String(StringExpr(text, QuoteKind::None), ..)
                if text.initial_plain().starts_with("--") =>
            {
                let text = self.parse_interpolated_declaration_value(false, false, true, false)?;
                AstExpr::String(
                    StringExpr(text, QuoteKind::None),
                    self.toks_mut().span_from(start),
                )
            }
            _ => {
                self.whitespace_with_newlines()?;
                self.parse_expression_with_newlines(None)?.node
            }
        };

        Ok(AstSupportsCondition::Declaration { name, value })
    }

    fn supports_condition_in_parens(&mut self) -> SassResult<AstSupportsCondition> {
        let start = self.toks().cursor();

        if self.looking_at_interpolated_identifier() {
            let identifier = self.parse_interpolated_identifier()?;
            let ident_span = self.toks_mut().span_from(start);

            if identifier
                .as_plain()
                .unwrap_or("")
                .eq_ignore_ascii_case("not")
            {
                return Err((r#""not" is not a valid identifier here."#, ident_span).into());
            }

            if self.scan_char('(') {
                let arguments =
                    self.parse_interpolated_declaration_value(true, true, true, true)?;
                self.expect_char(')')?;
                return Ok(AstSupportsCondition::Function {
                    name: identifier,
                    args: arguments,
                });
            } else if identifier.contents.len() != 1
                || !matches!(
                    identifier.contents.first(),
                    Some(InterpolationPart::Expr(..))
                )
            {
                return Err(("Expected @supports condition.", ident_span).into());
            } else {
                match identifier.contents.first() {
                    Some(InterpolationPart::Expr(e)) => {
                        return Ok(AstSupportsCondition::Interpolation(e.clone().node));
                    }
                    _ => unreachable!(),
                }
            }
        }

        self.expect_char('(')?;
        self.whitespace_with_newlines()?;

        if self.scan_identifier("not", false)? {
            self.whitespace_with_newlines()?;
            let condition = self.supports_condition_in_parens()?;
            self.expect_char(')')?;
            return Ok(AstSupportsCondition::Negation(Box::new(condition)));
        } else if self.toks_mut().next_char_is('(') {
            let condition = self.parse_supports_condition(true)?;
            self.expect_char(')')?;
            return Ok(condition);
        }

        // Unfortunately, we may have to backtrack here. The grammar is:
        //
        //       Expression ":" Expression
        //     | InterpolatedIdentifier InterpolatedAnyValue?
        //
        // These aren't ambiguous because this `InterpolatedAnyValue` is forbidden
        // from containing a top-level colon, but we still have to parse the full
        // expression to figure out if there's a colon after it.
        //
        // We could avoid the overhead of a full expression parse by looking ahead
        // for a colon (outside of balanced brackets), but in practice we expect the
        // vast majority of real uses to be `Expression ":" Expression`, so it makes
        // sense to parse that case faster in exchange for less code complexity and
        // a slower backtracking case.

        let name: AstExpr;
        let name_start = self.toks().cursor();
        let was_in_parens = self.flags().in_parens();

        let expr = self.parse_expression_with_newlines(None);
        let found_colon = self.expect_char(':');
        match (expr, found_colon) {
            (Ok(val), Ok(..)) => {
                name = val.node;
            }
            (Ok(..), Err(e)) | (Err(e), Ok(..)) | (Err(e), Err(..)) => {
                self.toks_mut().set_cursor(name_start);
                self.flags_mut().set(ContextFlags::IN_PARENS, was_in_parens);

                let identifier = self.parse_interpolated_identifier()?;

                // todo: superfluous clone?
                if let Some(operation) = self.try_supports_operation(&identifier, name_start)? {
                    self.expect_char(')')?;
                    return Ok(operation);
                }

                // If parsing an expression fails, try to parse an
                // `InterpolatedAnyValue` instead. But if that value runs into a
                // top-level colon, then this is probably intended to be a declaration
                // after all, so we rethrow the declaration-parsing error.
                let mut contents = Interpolation::new();
                contents.add_interpolation(identifier);
                contents.add_interpolation(
                    self.parse_interpolated_declaration_value(true, true, false, true)?,
                );

                if self.toks_mut().next_char_is(':') {
                    return Err(e);
                }

                self.expect_char(')')?;

                return Ok(AstSupportsCondition::Anything { contents });
            }
        }

        let declaration = self.supports_declaration_value(name, start)?;
        self.expect_char(')')?;

        Ok(declaration)
    }

    /// Parses a `@supports` condition. In parentheses (`in_parentheses`), newlines are whitespace in
    /// the indented syntax too.
    fn parse_supports_condition(
        &mut self,
        in_parentheses: bool,
    ) -> SassResult<AstSupportsCondition> {
        if self.scan_identifier("not", false)? {
            self.whitespace_newlines_if(in_parentheses)?;
            return Ok(AstSupportsCondition::Negation(Box::new(
                self.supports_condition_in_parens()?,
            )));
        }

        let mut condition = self.supports_condition_in_parens()?;
        self.whitespace_newlines_if(in_parentheses)?;

        let mut operator: Option<String> = None;

        while self.looking_at_identifier() {
            if let Some(operator) = &operator {
                self.expect_identifier(operator, false)?;
            } else if self.scan_identifier("or", false)? {
                operator = Some("or".to_owned());
            } else {
                self.expect_identifier("and", false)?;
                operator = Some("and".to_owned());
            }

            self.whitespace_newlines_if(in_parentheses)?;
            let right = self.supports_condition_in_parens()?;
            condition = AstSupportsCondition::Operation {
                left: Box::new(condition),
                operator: operator.clone(),
                right: Box::new(right),
            };
            self.whitespace_newlines_if(in_parentheses)?;
        }

        Ok(condition)
    }

    fn parse_supports_rule(&mut self) -> SassResult<AstStmt> {
        let condition = self.parse_supports_condition(false)?;
        self.whitespace()?;
        let children = self.with_children(Self::parse_statement)?;

        Ok(AstStmt::Supports(AstSupportsRule {
            condition,
            body: children.node,
            span: children.span,
        }))
    }
}

impl<'a, T: StylesheetParser<'a>> SupportsParser<'a> for T {}
