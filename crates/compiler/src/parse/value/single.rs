//! Single expressions: operations, parentheses, maps, variables and unary operators.

use super::*;

impl<'a, 'c, P: StylesheetParser<'a>> ValueParser<'a, 'c, P> {
    pub(super) fn parse_single_expression(
        &mut self,
        parser: &mut P,
    ) -> SassResult<Spanned<AstExpr>> {
        let start = parser.toks().cursor();
        let first = parser.toks().peek();

        match first {
            Some(Token { kind: '(', .. }) => self.parse_paren_expr(parser),
            Some(Token { kind: '/', .. }) => self.parse_unary_operation(parser),
            Some(Token { kind: '[', .. }) => {
                Self::parse_expression(parser, None, true, false, false)
            }
            Some(Token { kind: '$', .. }) => Self::parse_variable(parser),
            Some(Token { kind: '&', .. }) => Self::parse_selector(parser),
            Some(Token { kind: '"', .. }) | Some(Token { kind: '\'', .. }) => Ok(parser
                .parse_interpolated_string()?
                .map_node(|s| AstExpr::String(s, parser.toks_mut().span_from(start)))),
            Some(Token { kind: '#', .. }) => self.parse_hash(parser),
            Some(Token { kind: '+', .. }) => self.parse_plus_expr(parser),
            Some(Token { kind: '-', .. }) => self.parse_minus_expr(parser),
            Some(Token { kind: '!', .. }) => Self::parse_important_expr(parser),
            Some(Token { kind: 'u', .. }) | Some(Token { kind: 'U', .. }) => {
                if matches!(parser.toks().peek_n(1), Some(Token { kind: '+', .. })) {
                    Self::parse_unicode_range(parser)
                } else {
                    self.parse_identifier_like(parser)
                }
            }
            Some(Token {
                kind: '0'..='9', ..
            })
            | Some(Token { kind: '.', .. }) => ValueParser::parse_number(parser),
            Some(Token {
                kind: 'a'..='z', ..
            })
            | Some(Token {
                kind: 'A'..='Z', ..
            })
            | Some(Token { kind: '_', .. })
            | Some(Token { kind: '\\', .. })
            | Some(Token {
                kind: '\u{80}'..=std::char::MAX,
                ..
            }) => self.parse_identifier_like(parser),
            Some(..) | None => Err((
                "Expected expression.",
                parser.toks_mut().span_from(self.start),
            )
                .into()),
        }
    }

    pub(super) fn resolve_one_operation(&mut self, parser: &mut P) -> SassResult<()> {
        let operator = self.binary_operators.as_mut().unwrap().pop().unwrap();
        let whitespace_around_operator = self.operator_whitespace.pop().unwrap_or(true);
        let operands = self.operands.as_mut().unwrap();

        let left = operands.pop().unwrap();
        let right = match self.single_expression.take() {
            Some(val) => val,
            None => return Err(("Expected expression.", left.span).into()),
        };

        let span = left.span.merge(right.span);

        if self.allow_slash
            && !parser.flags().in_parens()
            && operator == BinaryOp::Div
            && left.node.is_slash_operand()
            && right.node.is_slash_operand()
        {
            self.single_expression = Some(AstExpr::slash(left.node, right.node, span).span(span));
        } else {
            self.single_expression = Some(
                AstExpr::BinaryOp(Arc::new(BinaryOpExpr {
                    lhs: left.node,
                    op: operator,
                    rhs: right.node,
                    allows_slash: false,
                    whitespace_around_operator,
                    span,
                }))
                .span(span),
            );
            self.allow_slash = false;
        }

        Ok(())
    }

    pub(super) fn resolve_operations(&mut self, parser: &mut P) -> SassResult<()> {
        loop {
            let should_break = match self.binary_operators.as_ref() {
                Some(bin) => bin.is_empty(),
                None => true,
            };

            if should_break {
                break;
            }

            self.resolve_one_operation(parser)?;
        }

        Ok(())
    }

    pub(super) fn add_single_expression(
        &mut self,
        expression: Spanned<AstExpr>,
        parser: &mut P,
    ) -> SassResult<()> {
        if self.single_expression.is_some() {
            // If we discover we're parsing a list whose first element is a division
            // operation, and we're in parentheses, reparse outside of a paren
            // context. This ensures that `(1/2 1)` doesn't perform division on its
            // first element.
            if parser.flags().in_parens() {
                parser.flags_mut().set(ContextFlags::IN_PARENS, false);

                if self.allow_slash {
                    self.reset_state(parser)?;

                    return Ok(());
                }
            }

            if self.space_expressions.is_none() {
                self.space_expressions = Some(Vec::new());
            }

            self.resolve_operations(parser)?;

            self.space_expressions
                .as_mut()
                .unwrap()
                .push(self.single_expression.take().unwrap());

            self.allow_slash = true;
        }

        self.single_expression = Some(expression);

        Ok(())
    }

    pub(super) fn parse_map(
        parser: &mut P,
        first: Spanned<AstExpr>,
        start: usize,
    ) -> SassResult<Spanned<AstExpr>> {
        let mut pairs = vec![(first, parser.parse_expression_until_comma(false)?.node)];

        while parser.scan_char(',') {
            parser.whitespace_with_newlines()?;
            if !parser.looking_at_expression() {
                break;
            }

            let key = parser.parse_expression_until_comma(false)?;
            parser.expect_char(':')?;
            parser.whitespace_with_newlines()?;
            let value = parser.parse_expression_until_comma(false)?;
            pairs.push((key, value.node));
        }

        parser.expect_char(')')?;

        Ok(AstExpr::Map(AstSassMap(pairs)).span(parser.toks_mut().span_from(start)))
    }

    pub(super) fn parse_paren_expr(&mut self, parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        let start = parser.toks().cursor();
        if parser.is_plain_css() {
            // Parentheses are only allowed within calculations, which the evaluator checks.
            parser.expect_char('(')?;
            parser.whitespace_with_newlines()?;
            let expr = parser.parse_expression_until_comma(false)?;
            parser.expect_char(')')?;
            return Ok(AstExpr::Paren(Arc::new(expr.node)).span(parser.toks_mut().span_from(start)));
        }

        let was_in_parentheses = parser.flags().in_parens();
        parser.flags_mut().set(ContextFlags::IN_PARENS, true);

        parser.expect_char('(')?;
        parser.whitespace_with_newlines()?;
        if !parser.looking_at_expression() {
            parser.expect_char(')')?;
            parser
                .flags_mut()
                .set(ContextFlags::IN_PARENS, was_in_parentheses);
            return Ok(AstExpr::List(ListExpr {
                elems: Vec::new(),
                separator: ListSeparator::Undecided,
                brackets: Brackets::None,
            })
            .span(parser.toks_mut().span_from(start)));
        }

        let first = parser.parse_expression_until_comma(false)?;
        if parser.scan_char(':') {
            parser.whitespace_with_newlines()?;
            parser
                .flags_mut()
                .set(ContextFlags::IN_PARENS, was_in_parentheses);
            return Self::parse_map(parser, first, start);
        }

        if !parser.scan_char(',') {
            parser.expect_char(')')?;
            parser
                .flags_mut()
                .set(ContextFlags::IN_PARENS, was_in_parentheses);
            return Ok(AstExpr::Paren(Arc::new(first.node)).span(first.span));
        }

        parser.whitespace_with_newlines()?;

        let mut expressions = vec![first];

        loop {
            if !parser.looking_at_expression() {
                break;
            }
            expressions.push(parser.parse_expression_until_comma(false)?);
            if !parser.scan_char(',') {
                break;
            }
            parser.whitespace_with_newlines()?;
        }

        parser.expect_char(')')?;

        parser
            .flags_mut()
            .set(ContextFlags::IN_PARENS, was_in_parentheses);

        Ok(AstExpr::List(ListExpr {
            elems: expressions,
            separator: ListSeparator::Comma,
            brackets: Brackets::None,
        })
        .span(parser.toks_mut().span_from(start)))
    }

    pub(super) fn parse_variable(parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        let start = parser.toks().cursor();
        let name = parser.parse_variable_name()?;

        if parser.is_plain_css() {
            return Err((
                "Sass variables aren't allowed in plain CSS.",
                parser.toks_mut().span_from(start),
            )
                .into());
        }

        Ok(AstExpr::Variable {
            name: Spanned {
                node: Identifier::from(name),
                span: parser.toks_mut().span_from(start),
            },
            namespace: None,
        }
        .span(parser.toks_mut().span_from(start)))
    }

    pub(super) fn parse_selector(parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        if parser.is_plain_css() {
            return Err((
                "The parent selector isn't allowed in plain CSS.",
                parser.toks().current_span(),
            )
                .into());
        }

        let start = parser.toks().cursor();

        parser.expect_char('&')?;

        if parser.toks().next_char_is('&') {
            // todo: emit a warning here
            //   warn(
            //       'In Sass, "&&" means two copies of the parent selector. You '
            //       'probably want to use "and" instead.',
            //       scanner.spanFrom(start));
            //   scanner.position--;
        }

        Ok(AstExpr::ParentSelector.span(parser.toks_mut().span_from(start)))
    }

    pub(super) fn parse_unary_operation(&mut self, parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        let op_span = parser.toks().current_span();
        let operator = Self::expect_unary_operator(parser)?;

        if parser.is_plain_css() && operator != UnaryOp::Div {
            return Err(("Operators aren't allowed in plain CSS.", op_span).into());
        }

        parser.whitespace_with_newlines()?;

        let operand = self.parse_single_expression(parser)?;

        let span = op_span.merge(parser.toks().current_span());

        Ok(AstExpr::UnaryOp(operator, Arc::new(operand.node), span).span(span))
    }

    pub(super) fn expect_unary_operator(parser: &mut P) -> SassResult<UnaryOp> {
        let span = parser.toks().current_span();
        Ok(match parser.toks_mut().next() {
            Some(Token { kind: '+', .. }) => UnaryOp::Plus,
            Some(Token { kind: '-', .. }) => UnaryOp::Neg,
            Some(Token { kind: '/', .. }) => UnaryOp::Div,
            Some(..) | None => return Err(("Expected unary operator.", span).into()),
        })
    }

    pub(super) fn parse_plus_expr(&mut self, parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        debug_assert!(parser.toks().next_char_is('+'));

        match parser.toks().peek_n(1) {
            Some(Token {
                kind: '0'..='9' | '.',
                ..
            }) => ValueParser::parse_number(parser),
            _ => self.parse_unary_operation(parser),
        }
    }

    pub(super) fn parse_minus_expr(&mut self, parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        debug_assert!(parser.toks().next_char_is('-'));

        if matches!(
            parser.toks().peek_n(1),
            Some(Token {
                kind: '0'..='9' | '.',
                ..
            })
        ) {
            return ValueParser::parse_number(parser);
        }

        if parser.looking_at_interpolated_identifier() {
            return self.parse_identifier_like(parser);
        }

        self.parse_unary_operation(parser)
    }

    pub(super) fn parse_important_expr(parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        let start = parser.toks().cursor();
        parser.expect_char('!')?;
        parser.whitespace_with_newlines()?;
        parser.expect_identifier("important", false)?;

        let span = parser.toks_mut().span_from(start);

        Ok(AstExpr::String(
            StringExpr(
                Interpolation::new_plain("!important".to_owned()),
                QuoteKind::None,
            ),
            span,
        )
        .span(span))
    }
}
