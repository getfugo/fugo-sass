//! `@media` queries.

use super::*;

/// `@media` queries.
pub(crate) trait MediaParser<'a>: StylesheetParser<'a> {
    fn expression_until_comparison(&mut self) -> SassResult<Spanned<AstExpr>> {
        self.parse_expression_with_newlines(Some(&|parser| {
            Ok(match parser.toks().peek() {
                Some(Token { kind: '>', .. }) | Some(Token { kind: '<', .. }) => true,
                Some(Token { kind: '=', .. }) => {
                    !matches!(parser.toks().peek_n(1), Some(Token { kind: '=', .. }))
                }
                _ => false,
            })
        }))
    }

    fn parse_media_query_list(&mut self) -> SassResult<Interpolation> {
        let mut buf = Interpolation::new();
        loop {
            self.whitespace()?;
            self.parse_media_query(&mut buf)?;
            self.whitespace()?;
            if !self.scan_char(',') {
                break;
            }
            buf.add_char(',');
            buf.add_char(' ');
        }
        Ok(buf)
    }

    fn parse_media_in_parens(&mut self, buf: &mut Interpolation) -> SassResult<()> {
        self.expect_char_with_message('(', "media condition in parentheses")?;
        buf.add_char('(');
        self.whitespace_with_newlines()?;

        if matches!(self.toks().peek(), Some(Token { kind: '(', .. })) {
            self.parse_media_in_parens(buf)?;
            self.whitespace_with_newlines()?;

            if self.scan_identifier("and", false)? {
                buf.add_string(" and ".to_owned());
                self.expect_whitespace_newlines_if(true)?;
                self.parse_media_logic_sequence(buf, "and")?;
            } else if self.scan_identifier("or", false)? {
                buf.add_string(" or ".to_owned());
                self.expect_whitespace_newlines_if(true)?;
                self.parse_media_logic_sequence(buf, "or")?;
            }
        } else if self.scan_identifier("not", false)? {
            buf.add_string("not ".to_owned());
            self.expect_whitespace_newlines_if(true)?;
            self.parse_media_or_interpolation(buf)?;
        } else {
            buf.add_expr(self.expression_until_comparison()?);

            if self.scan_char(':') {
                self.whitespace_with_newlines()?;
                buf.add_char(':');
                buf.add_char(' ');
                buf.add_expr(self.parse_expression_with_newlines(None)?);
            } else {
                let next = self.toks().peek();
                if matches!(
                    next,
                    Some(Token {
                        kind: '<' | '>' | '=',
                        ..
                    })
                ) {
                    let next = next.unwrap().kind;
                    buf.add_char(' ');
                    buf.add_char(self.toks_mut().next().unwrap().kind);

                    if (next == '<' || next == '>') && self.scan_char('=') {
                        buf.add_char('=');
                    }

                    buf.add_char(' ');

                    self.whitespace_with_newlines()?;

                    buf.add_expr(self.expression_until_comparison()?);

                    if (next == '<' || next == '>') && self.scan_char(next) {
                        buf.add_char(' ');
                        buf.add_char(next);

                        if self.scan_char('=') {
                            buf.add_char('=');
                        }

                        buf.add_char(' ');

                        self.whitespace_with_newlines()?;
                        buf.add_expr(self.expression_until_comparison()?);
                    }
                }
            }
        }

        self.expect_char(')')?;
        self.whitespace()?;
        buf.add_char(')');

        Ok(())
    }

    fn parse_media_logic_sequence(
        &mut self,
        buf: &mut Interpolation,
        operator: &'static str,
    ) -> SassResult<()> {
        loop {
            self.parse_media_or_interpolation(buf)?;
            self.whitespace()?;

            if !self.scan_identifier(operator, false)? {
                return Ok(());
            }

            self.expect_whitespace()?;

            buf.add_char(' ');
            buf.add_string(operator.to_owned());
            buf.add_char(' ');
        }
    }

    fn parse_media_or_interpolation(&mut self, buf: &mut Interpolation) -> SassResult<()> {
        if self.toks_mut().next_char_is('#') {
            buf.add_interpolation(self.parse_single_interpolation()?);
        } else {
            self.parse_media_in_parens(buf)?;
        }

        Ok(())
    }

    fn parse_media_query(&mut self, buf: &mut Interpolation) -> SassResult<()> {
        if matches!(self.toks().peek(), Some(Token { kind: '(', .. })) {
            self.parse_media_in_parens(buf)?;
            self.whitespace()?;

            if self.scan_identifier("and", false)? {
                buf.add_string(" and ".to_owned());
                self.expect_whitespace()?;
                self.parse_media_logic_sequence(buf, "and")?;
            } else if self.scan_identifier("or", false)? {
                buf.add_string(" or ".to_owned());
                self.expect_whitespace()?;
                self.parse_media_logic_sequence(buf, "or")?;
            }

            return Ok(());
        }

        let ident1 = self.parse_interpolated_identifier()?;

        if ident1.as_plain().unwrap_or("").eq_ignore_ascii_case("not") {
            // For example, "@media not (...) {"
            self.expect_whitespace()?;
            if !self.looking_at_interpolated_identifier() {
                buf.add_string("not ".to_owned());
                self.parse_media_or_interpolation(buf)?;
                return Ok(());
            }
        }

        self.whitespace()?;
        buf.add_interpolation(ident1);
        if !self.looking_at_interpolated_identifier() {
            // For example, "@media screen {".
            return Ok(());
        }

        buf.add_char(' ');

        let ident2 = self.parse_interpolated_identifier()?;

        if ident2.as_plain().unwrap_or("").eq_ignore_ascii_case("and") {
            self.expect_whitespace()?;
            // For example, "@media screen and ..."
            buf.add_string(" and ".to_owned());
        } else {
            self.whitespace()?;
            buf.add_interpolation(ident2);

            if self.scan_identifier("and", false)? {
                // For example, "@media only screen and ..."
                self.expect_whitespace()?;
                buf.add_string(" and ".to_owned());
            } else {
                // For example, "@media only screen {"
                return Ok(());
            }
        }

        // We've consumed either `IDENTIFIER "and"` or
        // `IDENTIFIER IDENTIFIER "and"`.

        if self.scan_identifier("not", false)? {
            // For example, "@media screen and not (...) {"
            self.expect_whitespace()?;
            buf.add_string("not ".to_owned());
            self.parse_media_or_interpolation(buf)?;
            return Ok(());
        }

        self.parse_media_logic_sequence(buf, "and")?;

        Ok(())
    }
}

impl<'a, T: StylesheetParser<'a>> MediaParser<'a> for T {}
