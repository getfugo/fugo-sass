//! The expression loop: operands, operators, and space- and comma-separated lists.

use super::*;

impl<'a, 'c, P: StylesheetParser<'a>> ValueParser<'a, 'c, P> {
    pub(crate) fn parse_value(&mut self, parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        self.whitespace(parser)?;

        let start = parser.toks().cursor();

        let was_in_parens = parser.flags().in_parens();

        loop {
            self.whitespace(parser)?;

            if let Some(parse_until) = self.parse_until
                && parse_until(parser)?
            {
                break;
            }

            let first = parser.toks().peek();

            match first {
                Some(Token { kind: '(', .. }) => {
                    let expr = self.parse_paren_expr(parser)?;
                    self.add_single_expression(expr, parser)?;
                }
                Some(Token { kind: '[', .. }) => {
                    let expr = parser.parse_expression(None, Some(true), None)?;
                    self.add_single_expression(expr, parser)?;
                }
                Some(Token { kind: '$', .. }) => {
                    let expr = Self::parse_variable(parser)?;
                    self.add_single_expression(expr, parser)?;
                }
                Some(Token { kind: '&', .. }) => {
                    let expr = Self::parse_selector(parser)?;
                    self.add_single_expression(expr, parser)?;
                }
                Some(Token { kind: '"', .. }) | Some(Token { kind: '\'', .. }) => {
                    let expr = parser
                        .parse_interpolated_string()?
                        .map_node(|s| AstExpr::String(s, parser.toks_mut().span_from(start)));
                    self.add_single_expression(expr, parser)?;
                }
                Some(Token { kind: '#', .. }) => {
                    let expr = self.parse_hash(parser)?;
                    self.add_single_expression(expr, parser)?;
                }
                Some(Token { kind: '=', .. }) => {
                    parser.toks_mut().next();
                    if self.single_equals
                        && !matches!(parser.toks().peek(), Some(Token { kind: '=', .. }))
                    {
                        self.add_operator(
                            Spanned {
                                node: BinaryOp::SingleEq,
                                span: parser.toks_mut().span_from(start),
                            },
                            parser,
                        )?;
                    } else {
                        parser.expect_char('=')?;
                        self.add_operator(
                            Spanned {
                                node: BinaryOp::Equal,
                                span: parser.toks_mut().span_from(start),
                            },
                            parser,
                        )?;
                    }
                }
                Some(Token { kind: '!', .. }) => match parser.toks().peek_n(1) {
                    Some(Token { kind: '=', .. }) => {
                        parser.toks_mut().next();
                        parser.toks_mut().next();
                        self.add_operator(
                            Spanned {
                                node: BinaryOp::NotEqual,
                                span: parser.toks_mut().span_from(start),
                            },
                            parser,
                        )?;
                    }
                    Some(Token { kind, .. })
                        if kind.is_ascii_whitespace() || kind == 'i' || kind == 'I' =>
                    {
                        let expr = Self::parse_important_expr(parser)?;
                        self.add_single_expression(expr, parser)?;
                    }
                    None => {
                        let expr = Self::parse_important_expr(parser)?;
                        self.add_single_expression(expr, parser)?;
                    }
                    Some(..) => break,
                },
                Some(Token { kind: '<', .. }) => {
                    parser.toks_mut().next();
                    self.add_operator(
                        Spanned {
                            node: if parser.scan_char('=') {
                                BinaryOp::LessThanEqual
                            } else {
                                BinaryOp::LessThan
                            },
                            span: parser.toks_mut().span_from(start),
                        },
                        parser,
                    )?;
                }
                Some(Token { kind: '>', .. }) => {
                    parser.toks_mut().next();
                    self.add_operator(
                        Spanned {
                            node: if parser.scan_char('=') {
                                BinaryOp::GreaterThanEqual
                            } else {
                                BinaryOp::GreaterThan
                            },
                            span: parser.toks_mut().span_from(start),
                        },
                        parser,
                    )?;
                }
                Some(Token { kind: '*', .. }) => {
                    parser.toks_mut().next();
                    self.add_operator(
                        Spanned {
                            node: BinaryOp::Mul,
                            span: parser.toks().current_span(),
                        },
                        parser,
                    )?;
                }
                Some(Token { kind: '+', .. }) => {
                    if self.single_expression.is_none() {
                        let expr = self.parse_unary_operation(parser)?;
                        self.add_single_expression(expr, parser)?;
                    } else {
                        parser.toks_mut().next();
                        self.add_operator(
                            Spanned {
                                node: BinaryOp::Plus,
                                span: parser.toks_mut().span_from(start),
                            },
                            parser,
                        )?;
                    }
                }
                Some(Token { kind: '-', .. }) => {
                    if matches!(
                        parser.toks().peek_n(1),
                        Some(Token {
                            kind: '0'..='9' | '.',
                            ..
                        })
                    ) && (self.single_expression.is_none()
                        || matches!(
                            parser.toks_mut().peek_previous(),
                            Some(Token {
                                kind: ' ' | '\t' | '\n' | '\r',
                                ..
                            })
                        ))
                    {
                        let expr = ValueParser::parse_number(parser)?;
                        self.add_single_expression(expr, parser)?;
                    } else if parser.looking_at_interpolated_identifier() {
                        let expr = self.parse_identifier_like(parser)?;
                        self.add_single_expression(expr, parser)?;
                    } else if self.single_expression.is_none() {
                        let expr = self.parse_unary_operation(parser)?;
                        self.add_single_expression(expr, parser)?;
                    } else {
                        parser.toks_mut().next();
                        self.add_operator(
                            Spanned {
                                node: BinaryOp::Minus,
                                span: parser.toks_mut().span_from(start),
                            },
                            parser,
                        )?;
                    }
                }
                Some(Token { kind: '/', .. }) => {
                    if self.single_expression.is_none() {
                        let expr = self.parse_unary_operation(parser)?;
                        self.add_single_expression(expr, parser)?;
                    } else {
                        parser.toks_mut().next();
                        self.add_operator(
                            Spanned {
                                node: BinaryOp::Div,
                                span: parser.toks_mut().span_from(start),
                            },
                            parser,
                        )?;
                    }
                }
                Some(Token { kind: '%', .. }) => {
                    parser.toks_mut().next();
                    self.add_operator(
                        Spanned {
                            node: BinaryOp::Rem,
                            span: parser.toks().current_span(),
                        },
                        parser,
                    )?;
                }
                Some(Token {
                    kind: '0'..='9', ..
                }) => {
                    let expr = ValueParser::parse_number(parser)?;
                    self.add_single_expression(expr, parser)?;
                }
                Some(Token { kind: '.', .. }) => {
                    if matches!(parser.toks().peek_n(1), Some(Token { kind: '.', .. })) {
                        break;
                    }
                    let expr = ValueParser::parse_number(parser)?;
                    self.add_single_expression(expr, parser)?;
                }
                Some(Token { kind: 'a', .. }) => {
                    if !parser.is_plain_css() && parser.scan_identifier("and", false)? {
                        self.add_operator(
                            Spanned {
                                node: BinaryOp::And,
                                span: parser.toks_mut().span_from(start),
                            },
                            parser,
                        )?;
                    } else {
                        let expr = self.parse_identifier_like(parser)?;
                        self.add_single_expression(expr, parser)?;
                    }
                }
                Some(Token { kind: 'o', .. }) => {
                    if !parser.is_plain_css() && parser.scan_identifier("or", false)? {
                        self.add_operator(
                            Spanned {
                                node: BinaryOp::Or,
                                span: parser.toks_mut().span_from(start),
                            },
                            parser,
                        )?;
                    } else {
                        let expr = self.parse_identifier_like(parser)?;
                        self.add_single_expression(expr, parser)?;
                    }
                }
                Some(Token { kind: 'u', .. }) | Some(Token { kind: 'U', .. }) => {
                    if matches!(parser.toks().peek_n(1), Some(Token { kind: '+', .. })) {
                        let expr = Self::parse_unicode_range(parser)?;
                        self.add_single_expression(expr, parser)?;
                    } else {
                        let expr = self.parse_identifier_like(parser)?;
                        self.add_single_expression(expr, parser)?;
                    }
                }
                Some(Token {
                    kind: 'b'..='z', ..
                })
                | Some(Token {
                    kind: 'A'..='Z', ..
                })
                | Some(Token { kind: '_', .. })
                | Some(Token { kind: '\\', .. })
                | Some(Token {
                    kind: '\u{80}'..=std::char::MAX,
                    ..
                }) => {
                    let expr = self.parse_identifier_like(parser)?;
                    self.add_single_expression(expr, parser)?;
                }
                Some(Token { kind: ',', .. }) => {
                    // If we discover we're parsing a list whose first element is a
                    // division operation, and we're in parentheses, reparse outside of a
                    // paren context. This ensures that `(1/2, 1)` doesn't perform division
                    // on its first element.
                    if parser.flags().in_parens() {
                        parser.flags_mut().set(ContextFlags::IN_PARENS, false);
                        if self.allow_slash {
                            self.reset_state(parser)?;
                            continue;
                        }
                        // todo: does this branch ever get hit
                    }

                    if self.single_expression.is_none() {
                        return Err(("Expected expression.", parser.toks().current_span()).into());
                    }

                    self.resolve_space_expressions(parser)?;

                    // [resolveSpaceExpressions] can modify [singleExpression_], but it
                    // can't set it to null`.
                    self.comma_expressions
                        .get_or_insert_with(Default::default)
                        .push(self.single_expression.take().unwrap());
                    parser.toks_mut().next();
                    self.allow_slash = true;
                }
                Some(..) | None => break,
            }
        }

        if self.inside_bracketed_list {
            parser.expect_char(']')?;
        }

        if self.comma_expressions.is_some() {
            self.resolve_space_expressions(parser)?;

            parser
                .flags_mut()
                .set(ContextFlags::IN_PARENS, was_in_parens);

            if let Some(single_expression) = self.single_expression.take() {
                self.comma_expressions
                    .as_mut()
                    .unwrap()
                    .push(single_expression);
            }

            Ok(AstExpr::List(ListExpr {
                elems: self.comma_expressions.take().unwrap(),
                separator: ListSeparator::Comma,
                brackets: if self.inside_bracketed_list {
                    Brackets::Bracketed
                } else {
                    Brackets::None
                },
            })
            .span(parser.toks_mut().span_from(start)))
        } else if self.inside_bracketed_list && self.space_expressions.is_some() {
            self.resolve_operations(parser)?;

            self.space_expressions
                .as_mut()
                .unwrap()
                .push(self.single_expression.take().unwrap());

            Ok(AstExpr::List(ListExpr {
                elems: self.space_expressions.take().unwrap(),
                separator: ListSeparator::Space,
                brackets: Brackets::Bracketed,
            })
            .span(parser.toks_mut().span_from(start)))
        } else {
            self.resolve_space_expressions(parser)?;

            if self.inside_bracketed_list {
                return Ok(AstExpr::List(ListExpr {
                    elems: vec![self.single_expression.take().unwrap()],
                    separator: ListSeparator::Undecided,
                    brackets: Brackets::Bracketed,
                })
                .span(parser.toks_mut().span_from(start)));
            }

            Ok(self.single_expression.take().unwrap())
        }
    }

    pub(super) fn add_operator(&mut self, op: Spanned<BinaryOp>, parser: &mut P) -> SassResult<()> {
        // `+`, `-`, `*` and `/` are allowed in calculations, which the evaluator checks.
        if parser.is_plain_css()
            && !matches!(
                op.node,
                BinaryOp::SingleEq
                    | BinaryOp::Plus
                    | BinaryOp::Minus
                    | BinaryOp::Mul
                    | BinaryOp::Div
            )
        {
            return Err(("Operators aren't allowed in plain CSS.", op.span).into());
        }

        self.allow_slash = self.allow_slash && op.node == BinaryOp::Div;

        if self.binary_operators.is_none() {
            self.binary_operators = Some(Vec::new());
        }

        if self.operands.is_none() {
            self.operands = Some(Vec::new());
        }

        while let Some(last_op) = self.binary_operators.as_ref().unwrap_or(&Vec::new()).last() {
            if last_op.precedence() < op.precedence() {
                break;
            }

            self.resolve_one_operation(parser)?;
        }
        self.binary_operators
            .get_or_insert_with(Default::default)
            .push(op.node);
        // The operator was just consumed: the character before it, and below the last one before
        // the right operand, must be whitespace or a comment's `/` (dart-sass reads the text
        // between the operands).
        let separates = |token: Option<Token>| {
            matches!(
                token,
                Some(Token {
                    kind: ' ' | '\t' | '\n' | '\r' | '\x0C' | '/',
                    ..
                })
            )
        };
        let before_operator = separates(parser.toks().peek_n_backwards(2));

        match self.single_expression.take() {
            Some(expr) => {
                self.operands.get_or_insert_with(Vec::new).push(expr);
            }
            None => return Err(("Expected expression.", op.span).into()),
        }

        parser.whitespace_with_newlines()?;
        self.operator_whitespace
            .push(before_operator && separates(parser.toks().peek_n_backwards(1)));

        self.single_expression = Some(self.parse_single_expression(parser)?);

        Ok(())
    }

    pub(super) fn resolve_space_expressions(&mut self, parser: &mut P) -> SassResult<()> {
        self.resolve_operations(parser)?;

        if let Some(mut space_expressions) = self.space_expressions.take() {
            let single_expression = match self.single_expression.take() {
                Some(val) => val,
                None => return Err(("Expected expression.", parser.toks().current_span()).into()),
            };

            let span = single_expression.span;

            space_expressions.push(single_expression);

            self.single_expression = Some(
                AstExpr::List(ListExpr {
                    elems: space_expressions,
                    separator: ListSeparator::Space,
                    brackets: Brackets::None,
                })
                .span(span),
            );
        }

        Ok(())
    }
}
