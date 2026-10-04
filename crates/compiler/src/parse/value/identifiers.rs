//! Expressions that start like an identifier: functions, keywords, colors, namespaces and special functions.

use super::*;

impl<'a, 'c, P: StylesheetParser<'a>> ValueParser<'a, 'c, P> {
    pub(super) fn parse_identifier_like(&mut self, parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        if let Some(func) = P::IDENTIFIER_LIKE {
            return func(parser);
        }

        let start = parser.toks().cursor();

        let identifier = parser.parse_interpolated_identifier()?;

        let ident_span = parser.toks_mut().span_from(start);

        let plain = identifier.as_plain();
        let lower = plain.map(str::to_ascii_lowercase);

        if let Some(plain) = plain {
            if plain == "if" && parser.toks().next_char_is('(') {
                // The legacy `if($condition, $if-true, $if-false)` and the CSS `if(condition:
                // value)` can only be told apart by parsing, so try the former first (as
                // dart-sass does).
                let before_paren = parser.toks().cursor();
                let flags = *parser.flags();
                match parser.parse_argument_invocation(false, false) {
                    Ok(call_args) => {
                        let span = parser.toks_mut().span_from(start);
                        return Ok(AstExpr::LegacyIf(Arc::new(Ternary(call_args))).span(span));
                    }
                    Err(..) => {
                        parser.toks_mut().set_cursor(before_paren);
                        *parser.flags_mut() = flags;
                        return parser.parse_if_expression(start);
                    }
                }
            } else if plain.eq_ignore_ascii_case("if") && parser.toks().next_char_is('(') {
                return parser.parse_if_expression(start);
            } else if plain == "not" {
                parser.whitespace_with_newlines()?;

                let value = self.parse_single_expression(parser)?;

                let span = parser.toks_mut().span_from(start);

                return Ok(AstExpr::UnaryOp(UnaryOp::Not, Arc::new(value.node), span).span(span));
            }

            let lower_ref = lower.as_ref().unwrap();

            if !parser.toks().next_char_is('(') {
                match plain {
                    "null" => return Ok(AstExpr::Null.span(parser.toks_mut().span_from(start))),
                    "true" => return Ok(AstExpr::True.span(parser.toks_mut().span_from(start))),
                    "false" => return Ok(AstExpr::False.span(parser.toks_mut().span_from(start))),
                    _ => {}
                }

                if let Some(color) = NAMED_COLORS.get_by_name(lower_ref.as_str()) {
                    return Ok(AstExpr::Color(Arc::new(Color::new(
                        color[0],
                        color[1],
                        color[2],
                        color[3],
                        plain.to_owned(),
                    )))
                    .span(parser.toks_mut().span_from(start)));
                }
            }

            if let Some(func) = ValueParser::try_parse_special_function(parser, lower_ref, start)? {
                return Ok(func);
            }
        }

        match parser.toks().peek() {
            Some(Token { kind: '.', .. }) => {
                if matches!(parser.toks().peek_n(1), Some(Token { kind: '.', .. })) {
                    return Ok(AstExpr::String(
                        StringExpr(identifier, QuoteKind::None),
                        parser.toks_mut().span_from(start),
                    )
                    .span(parser.toks_mut().span_from(start)));
                }
                parser.toks_mut().next();

                match plain {
                    Some(s) => Self::namespaced_expression(
                        Spanned {
                            node: Identifier::namespace(s),
                            span: ident_span,
                        },
                        start,
                        parser,
                    ),
                    None => Err(("Interpolation isn't allowed in namespaces.", ident_span).into()),
                }
            }
            Some(Token { kind: '(', .. }) => {
                if let Some(plain) = plain {
                    let arguments =
                        parser.parse_argument_invocation(false, lower.as_deref() == Some("var"))?;

                    Ok(AstExpr::FunctionCall(FunctionCallExpr {
                        namespace: None,
                        name: Identifier::from(plain),
                        arguments: Arc::new(arguments),
                        span: parser.toks_mut().span_from(start),
                    })
                    .span(parser.toks_mut().span_from(start)))
                } else {
                    let arguments = parser.parse_argument_invocation(false, false)?;
                    Ok(
                        AstExpr::InterpolatedFunction(Arc::new(InterpolatedFunction {
                            name: identifier,
                            arguments,
                            span: parser.toks_mut().span_from(start),
                        }))
                        .span(parser.toks_mut().span_from(start)),
                    )
                }
            }
            _ => Ok(AstExpr::String(
                StringExpr(identifier, QuoteKind::None),
                parser.toks_mut().span_from(start),
            )
            .span(parser.toks_mut().span_from(start))),
        }
    }

    pub(super) fn namespaced_expression(
        namespace: Spanned<Identifier>,
        start: usize,
        parser: &mut P,
    ) -> SassResult<Spanned<AstExpr>> {
        if parser.toks().next_char_is('$') {
            let name_start = parser.toks().cursor();
            let name = parser.parse_variable_name()?;
            let span = parser.toks_mut().span_from(start);
            P::assert_public(&name, span)?;

            if parser.is_plain_css() {
                return Err(("Module namespaces aren't allowed in plain CSS.", span).into());
            }

            return Ok(AstExpr::Variable {
                name: Spanned {
                    node: Identifier::from(name),
                    span: parser.toks_mut().span_from(name_start),
                },
                namespace: Some(namespace),
            }
            .span(span));
        }

        let name = parser.parse_public_identifier()?;
        let args = parser.parse_argument_invocation(false, false)?;
        let span = parser.toks_mut().span_from(start);

        if parser.is_plain_css() {
            return Err(("Module namespaces aren't allowed in plain CSS.", span).into());
        }

        Ok(AstExpr::FunctionCall(FunctionCallExpr {
            namespace: Some(namespace),
            name: Identifier::from(name),
            arguments: Arc::new(args),
            span,
        })
        .span(span))
    }

    /// If `name` (lowercase) is a function with special syntax, consumes it (dart-sass's
    /// `trySpecialFunction`). Calculations are ordinary function calls, evaluated as
    /// calculations when no Sass function of the name exists; only `-*-calc()` keeps its old
    /// special syntax.
    pub(crate) fn try_parse_special_function(
        parser: &mut P,
        name: &str,
        start: usize,
    ) -> SassResult<Option<Spanned<AstExpr>>> {
        let mut buffer;

        if name == "type" && parser.scan_char('(') {
            buffer = Interpolation::new_plain(name.to_owned());
            buffer.add_char('(');
        } else {
            let normalized = unvendor(name);
            let vendored = normalized != name;

            match normalized {
                "calc" if vendored && parser.toks().next_char_is('(') => {
                    parser.expect_char('(')?;
                    buffer = Interpolation::new_plain(name.to_owned());
                    buffer.add_char('(');
                }
                "expression" | "element" if parser.toks().next_char_is('(') => {
                    parser.expect_char('(')?;
                    buffer = Interpolation::new_plain(name.to_owned());
                    buffer.add_char('(');
                }
                "progid" => {
                    if !parser.scan_char(':') {
                        return Ok(None);
                    }
                    buffer = Interpolation::new_plain(name.to_owned());
                    buffer.add_char(':');

                    while let Some(Token { kind, .. }) = parser.toks().peek() {
                        if !kind.is_alphabetic() && kind != '.' {
                            break;
                        }
                        buffer.add_char(kind);
                        parser.toks_mut().next();
                    }
                    parser.expect_char('(')?;
                    buffer.add_char('(');
                }
                "url" => {
                    return Ok(parser.try_url_contents(None)?.map(|contents| {
                        AstExpr::String(
                            StringExpr(contents, QuoteKind::None),
                            parser.toks_mut().span_from(start),
                        )
                        .span(parser.toks_mut().span_from(start))
                    }));
                }
                _ => return Ok(None),
            }
        }

        buffer.add_interpolation(
            parser.parse_interpolated_declaration_value(false, true, true, false)?,
        );
        parser.expect_char(')')?;
        buffer.add_char(')');

        Ok(Some(
            AstExpr::String(
                StringExpr(buffer, QuoteKind::None),
                parser.toks_mut().span_from(start),
            )
            .span(parser.toks_mut().span_from(start)),
        ))
    }
}
