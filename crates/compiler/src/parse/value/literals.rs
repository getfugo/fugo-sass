//! Literals: numbers, hex colors and unicode ranges.

use super::*;

impl<'a, 'c, P: StylesheetParser<'a>> ValueParser<'a, 'c, P> {
    pub(super) fn parse_hash(&mut self, parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        let start = parser.toks().cursor();
        debug_assert!(matches!(
            parser.toks().peek(),
            Some(Token { kind: '#', .. })
        ));

        if matches!(parser.toks().peek_n(1), Some(Token { kind: '{', .. })) {
            return self.parse_identifier_like(parser);
        }

        parser.expect_char('#')?;

        if matches!(
            parser.toks().peek(),
            Some(Token {
                kind: '0'..='9',
                ..
            })
        ) {
            let color = self.parse_hex_color_contents(parser)?;
            return Ok(AstExpr::Color(Arc::new(color)).span(parser.toks_mut().span_from(start)));
        }

        let after_hash = parser.toks().cursor();
        let ident = parser.parse_interpolated_identifier()?;
        if is_hex_color(&ident) {
            parser.toks_mut().set_cursor(after_hash);
            let color = self.parse_hex_color_contents(parser)?;
            return Ok(
                AstExpr::Color(Arc::new(color)).span(parser.toks_mut().span_from(after_hash))
            );
        }

        let mut buffer = Interpolation::new();

        buffer.add_char('#');
        buffer.add_interpolation(ident);

        let span = parser.toks_mut().span_from(start);

        Ok(AstExpr::String(StringExpr(buffer, QuoteKind::None), span).span(span))
    }

    pub(super) fn parse_hex_digit(&mut self, parser: &mut P) -> SassResult<u32> {
        match parser.toks().peek() {
            Some(Token { kind, .. }) if kind.is_ascii_hexdigit() => {
                parser.toks_mut().next();
                Ok(as_hex(kind))
            }
            _ => Err(("Expected hex digit.", parser.toks().current_span()).into()),
        }
    }

    pub(super) fn parse_hex_color_contents(&mut self, parser: &mut P) -> SassResult<Color> {
        let start = parser.toks().cursor();

        let digit1 = self.parse_hex_digit(parser)?;
        let digit2 = self.parse_hex_digit(parser)?;
        let digit3 = self.parse_hex_digit(parser)?;

        let red: u32;
        let green: u32;
        let blue: u32;
        let mut alpha: f64 = 1.0;

        if parser.next_is_hex() {
            let digit4 = self.parse_hex_digit(parser)?;

            if parser.next_is_hex() {
                red = (digit1 << 4) + digit2;
                green = (digit3 << 4) + digit4;
                blue = (self.parse_hex_digit(parser)? << 4) + self.parse_hex_digit(parser)?;

                if parser.next_is_hex() {
                    alpha = ((self.parse_hex_digit(parser)? << 4) + self.parse_hex_digit(parser)?)
                        as f64
                        / 0xff as f64;
                }
            } else {
                // #abcd
                red = (digit1 << 4) + digit1;
                green = (digit2 << 4) + digit2;
                blue = (digit3 << 4) + digit3;
                alpha = ((digit4 << 4) + digit4) as f64 / 0xff as f64;
            }
        } else {
            // #abc
            red = (digit1 << 4) + digit1;
            green = (digit2 << 4) + digit2;
            blue = (digit3 << 4) + digit3;
        }

        Ok(Color::new_rgba(
            Number::from(red),
            Number::from(green),
            Number::from(blue),
            Number(alpha),
            // todo:
            //     // Don't emit four- or eight-digit hex colors as hex, since that's not
            //     // yet well-supported in browsers.
            ColorFormat::Literal(parser.toks_mut().raw_text(start - 1)),
        ))
    }

    pub(super) fn consume_natural_number(parser: &mut P) -> SassResult<()> {
        if !matches!(
            parser.toks_mut().next(),
            Some(Token {
                kind: '0'..='9',
                ..
            })
        ) {
            return Err(("Expected digit.", parser.toks().prev_span()).into());
        }

        while matches!(
            parser.toks().peek(),
            Some(Token {
                kind: '0'..='9',
                ..
            })
        ) {
            parser.toks_mut().next();
        }

        Ok(())
    }

    pub(super) fn parse_number(parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        let start = parser.toks().cursor();

        if !parser.scan_char('+') {
            parser.scan_char('-');
        }

        let after_sign = parser.toks().cursor();

        if !parser.toks().next_char_is('.') {
            ValueParser::consume_natural_number(parser)?;
        }

        ValueParser::try_decimal(parser, parser.toks().cursor() != after_sign)?;
        ValueParser::try_exponent(parser)?;

        let number: f64 = parser.toks_mut().raw_text(start).parse().unwrap();

        let unit = if parser.scan_char('%') {
            Unit::Percent
        } else if parser.looking_at_identifier()
            && (!matches!(parser.toks().peek(), Some(Token { kind: '-', .. }))
                || !matches!(parser.toks().peek_n(1), Some(Token { kind: '-', .. })))
        {
            Unit::from(parser.parse_identifier(false, true)?)
        } else {
            Unit::None
        };

        Ok(AstExpr::Number {
            n: Number::from(number),
            unit,
        }
        .span(parser.toks_mut().span_from(start)))
    }

    pub(super) fn try_decimal(
        parser: &mut P,
        allow_trailing_dot: bool,
    ) -> SassResult<Option<String>> {
        if !matches!(parser.toks().peek(), Some(Token { kind: '.', .. })) {
            return Ok(None);
        }

        match parser.toks().peek_n(1) {
            Some(Token { kind, .. }) if !kind.is_ascii_digit() => {
                if allow_trailing_dot {
                    return Ok(None);
                }

                return Err(("Expected digit.", parser.toks().current_span()).into());
            }
            Some(..) => {}
            None => return Err(("Expected digit.", parser.toks().current_span()).into()),
        }

        let mut buffer = String::new();

        parser.expect_char('.')?;
        buffer.push('.');

        while let Some(Token { kind, .. }) = parser.toks().peek() {
            if !kind.is_ascii_digit() {
                break;
            }
            buffer.push(kind);
            parser.toks_mut().next();
        }

        Ok(Some(buffer))
    }

    pub(super) fn try_exponent(parser: &mut P) -> SassResult<Option<String>> {
        let mut buffer = String::new();

        match parser.toks().peek() {
            Some(Token {
                kind: 'e' | 'E', ..
            }) => buffer.push('e'),
            _ => return Ok(None),
        }

        let next = match parser.toks().peek_n(1) {
            Some(Token {
                kind: kind @ ('0'..='9' | '-' | '+'),
                ..
            }) => kind,
            _ => return Ok(None),
        };

        parser.toks_mut().next();

        if next == '+' || next == '-' {
            parser.toks_mut().next();
            buffer.push(next);
        }

        match parser.toks().peek() {
            Some(Token {
                kind: '0'..='9', ..
            }) => {}
            _ => return Err(("Expected digit.", parser.toks().current_span()).into()),
        }

        while let Some(tok) = parser.toks().peek() {
            if !tok.kind.is_ascii_digit() {
                break;
            }

            buffer.push(tok.kind);

            parser.toks_mut().next();
        }

        Ok(Some(buffer))
    }

    pub(super) fn parse_unicode_range(parser: &mut P) -> SassResult<Spanned<AstExpr>> {
        let start = parser.toks().cursor();
        parser.expect_ident_char('u', false)?;
        parser.expect_char('+')?;

        let mut first_range_length = 0;

        while let Some(next) = parser.toks().peek() {
            if !next.kind.is_ascii_hexdigit() {
                break;
            }

            parser.toks_mut().next();
            first_range_length += 1;
        }

        let mut has_question_mark = false;

        while parser.scan_char('?') {
            has_question_mark = true;
            first_range_length += 1;
        }

        let span = parser.toks_mut().span_from(start);
        if first_range_length == 0 {
            return Err(("Expected hex digit or \"?\".", parser.toks().current_span()).into());
        } else if first_range_length > 6 {
            return Err(("Expected at most 6 digits.", span).into());
        } else if has_question_mark {
            return Ok(AstExpr::String(
                StringExpr(
                    Interpolation::new_plain(parser.toks_mut().raw_text(start)),
                    QuoteKind::None,
                ),
                span,
            )
            .span(span));
        }

        if parser.scan_char('-') {
            let second_range_start = parser.toks().cursor();
            let mut second_range_length = 0;

            while let Some(next) = parser.toks().peek() {
                if !next.kind.is_ascii_hexdigit() {
                    break;
                }

                parser.toks_mut().next();
                second_range_length += 1;
            }

            if second_range_length == 0 {
                return Err(("Expected hex digit.", parser.toks().current_span()).into());
            } else if second_range_length > 6 {
                return Err((
                    "Expected at most 6 digits.",
                    parser.toks_mut().span_from(second_range_start),
                )
                    .into());
            }
        }

        if parser.looking_at_interpolated_identifier_body() {
            return Err(("Expected end of identifier.", parser.toks().current_span()).into());
        }

        let span = parser.toks_mut().span_from(start);

        Ok(AstExpr::String(
            StringExpr(
                Interpolation::new_plain(parser.toks_mut().raw_text(start)),
                QuoteKind::None,
            ),
            span,
        )
        .span(span))
    }
}
