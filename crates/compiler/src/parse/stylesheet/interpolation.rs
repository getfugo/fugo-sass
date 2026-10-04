//! Interpolated strings and identifiers.

use super::*;

/// Interpolated strings and identifiers.
pub(crate) trait InterpolationParser<'a>: StylesheetParser<'a> {
    fn parse_interpolated_string(&mut self) -> SassResult<Spanned<StringExpr>> {
        let start = self.toks().cursor();
        let quote = match self.toks_mut().next() {
            Some(Token {
                kind: kind @ ('"' | '\''),
                ..
            }) => kind,
            Some(..) | None => unreachable!("Expected string."),
        };

        let mut buffer = Interpolation::new();

        let mut found_match = false;

        while let Some(next) = self.toks().peek() {
            match next.kind {
                c if c == quote => {
                    self.toks_mut().next();
                    found_match = true;
                    break;
                }
                '\n' => break,
                '\\' => {
                    match self.toks().peek_n(1) {
                        // todo: if (second == $cr) scanner.scanChar($lf);
                        // we basically need to stop normalizing to gain parity
                        Some(Token { kind: '\n', .. }) => {
                            self.toks_mut().next();
                            self.toks_mut().next();
                        }
                        _ => buffer.add_char(self.consume_escaped_char()?),
                    }
                }
                '#' => {
                    if matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })) {
                        buffer.add_interpolation(self.parse_single_interpolation()?);
                    } else {
                        self.toks_mut().next();
                        buffer.add_char(next.kind);
                    }
                }
                _ => {
                    buffer.add_char(next.kind);
                    self.toks_mut().next();
                }
            }
        }

        if !found_match {
            return Err((
                format!("Expected {quote}.", quote = quote),
                self.toks().current_span(),
            )
                .into());
        }

        Ok(Spanned {
            node: StringExpr(buffer, QuoteKind::Quoted),
            span: self.toks_mut().span_from(start),
        })
    }

    fn parse_single_interpolation(&mut self) -> SassResult<Interpolation> {
        self.expect_char('#')?;
        self.expect_char('{')?;
        self.whitespace_with_newlines()?;
        let contents = self.parse_expression_with_newlines(None)?;
        self.expect_char('}')?;

        if self.is_plain_css() {
            return Err(("Interpolation isn't allowed in plain CSS.", contents.span).into());
        }

        let mut interpolation = Interpolation::new();
        interpolation
            .contents
            .push(InterpolationPart::Expr(contents));

        Ok(interpolation)
    }

    fn parse_interpolated_identifier_body(&mut self, buffer: &mut Interpolation) -> SassResult<()> {
        while let Some(next) = self.toks().peek() {
            match next.kind {
                'a'..='z' | 'A'..='Z' | '0'..='9' | '_' | '-' | '\u{80}'..=std::char::MAX => {
                    buffer.add_char(next.kind);
                    self.toks_mut().next();
                }
                '\\' => {
                    buffer.add_string(self.parse_escape(false)?);
                }
                '#' if matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })) => {
                    buffer.add_interpolation(self.parse_single_interpolation()?);
                }
                _ => break,
            }
        }

        Ok(())
    }

    fn parse_interpolated_identifier(&mut self) -> SassResult<Interpolation> {
        let mut buffer = Interpolation::new();

        if self.scan_char('-') {
            buffer.add_char('-');

            if self.scan_char('-') {
                buffer.add_char('-');
                self.parse_interpolated_identifier_body(&mut buffer)?;
                return Ok(buffer);
            }
        }

        match self.toks().peek() {
            Some(tok) if is_name_start(tok.kind) => {
                buffer.add_char(tok.kind);
                self.toks_mut().next();
            }
            Some(Token { kind: '\\', .. }) => {
                buffer.add_string(self.parse_escape(true)?);
            }
            Some(Token { kind: '#', .. })
                if matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })) =>
            {
                buffer.add_interpolation(self.parse_single_interpolation()?);
            }
            Some(..) | None => {
                return Err(("Expected identifier.", self.toks().current_span()).into());
            }
        }

        self.parse_interpolated_identifier_body(&mut buffer)?;

        Ok(buffer)
    }

    fn looking_at_interpolated_identifier(&mut self) -> bool {
        let first = match self.toks().peek() {
            Some(Token { kind: '\\', .. }) => return true,
            Some(Token { kind: '#', .. }) => {
                return matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. }));
            }
            Some(Token { kind, .. }) if is_name_start(kind) => return true,
            Some(tok) => tok,
            None => return false,
        };

        if first.kind != '-' {
            return false;
        }

        match self.toks().peek_n(1) {
            Some(Token { kind: '#', .. }) => {
                matches!(self.toks().peek_n(2), Some(Token { kind: '{', .. }))
            }
            Some(Token {
                kind: '\\' | '-', ..
            }) => true,
            Some(Token { kind, .. }) => is_name_start(kind),
            None => false,
        }
    }

    fn next_is_hex(&self) -> bool {
        match self.toks().peek() {
            Some(Token { kind, .. }) => kind.is_ascii_hexdigit(),
            None => false,
        }
    }

    fn looking_at_interpolated_identifier_body(&mut self) -> bool {
        match self.toks().peek() {
            Some(Token { kind: '\\', .. }) => true,
            Some(Token { kind: '#', .. })
                if matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })) =>
            {
                true
            }
            Some(Token { kind, .. }) if is_name(kind) => true,
            Some(..) | None => false,
        }
    }
}

impl<'a, T: StylesheetParser<'a>> InterpolationParser<'a> for T {}
