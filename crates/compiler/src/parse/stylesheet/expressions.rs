//! Expressions and argument lists.

use super::*;

/// Expressions and argument lists.
pub(crate) trait ExpressionParser<'a>: StylesheetParser<'a> {
    fn looking_at_expression(&mut self) -> bool {
        let character = if let Some(c) = self.toks().peek() {
            c
        } else {
            return false;
        };

        match character.kind {
            '.' => !matches!(self.toks().peek_n(1), Some(Token { kind: '.', .. })),
            '!' => match self.toks().peek_n(1) {
                Some(Token {
                    kind: 'i' | 'I', ..
                })
                | None => true,
                Some(Token { kind, .. }) => kind.is_ascii_whitespace(),
            },
            '(' | '/' | '[' | '\'' | '"' | '#' | '+' | '-' | '\\' | '$' | '&' => true,
            c => is_name_start(c) || c.is_ascii_digit(),
        }
    }

    fn parse_expression_until_comma(
        &mut self,
        // default=false
        single_equals: bool,
    ) -> SassResult<Spanned<AstExpr>> {
        ValueParser::parse_expression(
            self,
            Some(&|parser| {
                Ok(matches!(
                    parser.toks().peek(),
                    Some(Token { kind: ',', .. })
                ))
            }),
            false,
            single_equals,
            true,
        )
    }

    fn parse_argument_invocation(
        &mut self,
        for_mixin: bool,
        allow_empty_second_arg: bool,
    ) -> SassResult<ArgumentInvocation> {
        let start = self.toks().cursor();

        self.expect_char('(')?;
        self.whitespace_with_newlines()?;

        let mut positional = Vec::new();
        let mut named = IndexMap::new();

        let mut rest: Option<AstExpr> = None;
        let mut keyword_rest: Option<AstExpr> = None;

        while self.looking_at_expression() {
            let expression = self.parse_expression_until_comma(!for_mixin)?;
            self.whitespace_with_newlines()?;

            if expression.node.is_variable() && self.scan_char(':') {
                let name = match expression.node {
                    AstExpr::Variable { name, .. } => name,
                    _ => unreachable!(),
                };

                self.whitespace_with_newlines()?;
                if named.contains_key(&name.node) {
                    return Err(("Duplicate argument.", name.span).into());
                }

                named.insert(
                    name.node,
                    self.parse_expression_until_comma(!for_mixin)?.node,
                );
            } else if self.scan_char('.') {
                self.expect_char('.')?;
                self.expect_char('.')?;

                if rest.is_none() {
                    rest = Some(expression.node);
                } else {
                    keyword_rest = Some(expression.node);
                    self.whitespace_with_newlines()?;
                    if self.scan_char(',') {
                        self.whitespace_with_newlines()?;
                    }
                    break;
                }
            } else if !named.is_empty() {
                return Err((
                    "Positional arguments must come before keyword arguments.",
                    expression.span,
                )
                    .into());
            } else {
                positional.push(expression.node);
            }

            self.whitespace_with_newlines()?;
            if !self.scan_char(',') {
                break;
            }
            self.whitespace_with_newlines()?;

            if allow_empty_second_arg
                && positional.len() == 1
                && named.is_empty()
                && rest.is_none()
                && matches!(self.toks().peek(), Some(Token { kind: ')', .. }))
            {
                positional.push(AstExpr::String(
                    StringExpr(Interpolation::new(), QuoteKind::None),
                    self.toks().current_span(),
                ));
                break;
            }
        }

        self.expect_char(')')?;

        Ok(ArgumentInvocation {
            positional,
            named,
            rest,
            keyword_rest,
            span: self.toks_mut().span_from(start),
        })
    }

    fn parse_expression(
        &mut self,
        parse_until: Option<Predicate<'_, Self>>,
        inside_bracketed_list: Option<bool>,
        single_equals: Option<bool>,
    ) -> SassResult<Spanned<AstExpr>> {
        ValueParser::parse_expression(
            self,
            parse_until,
            inside_bracketed_list.unwrap_or(false),
            single_equals.unwrap_or(false),
            false,
        )
    }

    /// `parse_expression(..)` for positions where the statement can't end, so that newlines are
    /// whitespace in the indented syntax too (dart-sass's `_expression(consumeNewlines: true)`).
    fn parse_expression_with_newlines(
        &mut self,
        parse_until: Option<Predicate<'_, Self>>,
    ) -> SassResult<Spanned<AstExpr>> {
        ValueParser::parse_expression(self, parse_until, false, false, true)
    }
}

impl<'a, T: StylesheetParser<'a>> ExpressionParser<'a> for T {}
