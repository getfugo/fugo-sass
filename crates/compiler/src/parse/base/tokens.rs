//! Strings, URLs and declaration values.

use super::*;

/// Strings, URLs and declaration values.
pub(crate) trait TokenParser: BaseParser {
    fn parse_string(&mut self) -> SassResult<String> {
        let quote = match self.toks_mut().next() {
            Some(Token {
                kind: q @ ('\'' | '"'),
                ..
            }) => q,
            Some(..) | None => return Err(("Expected string.", self.toks().current_span()).into()),
        };

        let mut buffer = String::new();

        let mut found_matching_quote = false;

        while let Some(next) = self.toks().peek() {
            if next.kind == quote {
                self.toks_mut().next();
                found_matching_quote = true;
                break;
            } else if next.kind == '\n' || next.kind == '\r' {
                break;
            } else if next.kind == '\\' {
                if matches!(
                    self.toks().peek_n(1),
                    Some(Token {
                        kind: '\n' | '\r',
                        ..
                    })
                ) {
                    self.toks_mut().next();
                    self.toks_mut().next();
                } else {
                    buffer.push(self.consume_escaped_char()?);
                }
            } else {
                self.toks_mut().next();
                buffer.push(next.kind);
            }
        }

        if !found_matching_quote {
            return Err((
                format!("Expected {quote}.", quote = quote),
                self.toks().current_span(),
            )
                .into());
        }

        Ok(buffer)
    }

    fn declaration_value(&mut self, allow_empty: bool) -> SassResult<String> {
        let mut buffer = String::new();

        let mut brackets = Vec::new();
        let mut wrote_newline = false;

        while let Some(tok) = self.toks().peek() {
            match tok.kind {
                '\\' => {
                    buffer.push_str(&self.parse_escape(true)?);
                    wrote_newline = false;
                }
                '"' | '\'' => {
                    buffer.push_str(&self.fallible_raw_text(Self::parse_string)?);
                    wrote_newline = false;
                }
                '/' => {
                    if matches!(self.toks().peek_n(1), Some(Token { kind: '*', .. })) {
                        buffer.push_str(&self.fallible_raw_text(Self::skip_loud_comment)?);
                    } else {
                        buffer.push('/');
                        self.toks_mut().next();
                    }

                    wrote_newline = false;
                }
                '#' => {
                    if matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })) {
                        let s = self.parse_identifier(false, false)?;
                        buffer.push_str(&s);
                    } else {
                        buffer.push('#');
                        self.toks_mut().next();
                    }

                    wrote_newline = false;
                }
                c @ (' ' | '\t') => {
                    if wrote_newline
                        || !self
                            .toks()
                            .peek_n(1)
                            .is_some_and(|tok| tok.kind.is_ascii_whitespace())
                    {
                        buffer.push(c);
                    }

                    self.toks_mut().next();
                }
                '\n' | '\r' => {
                    if !wrote_newline {
                        buffer.push('\n');
                    }

                    wrote_newline = true;

                    self.toks_mut().next();
                }

                '[' | '(' | '{' => {
                    buffer.push(tok.kind);

                    self.toks_mut().next();

                    brackets.push(opposite_bracket(tok.kind));
                    wrote_newline = false;
                }
                ']' | ')' | '}' => {
                    if let Some(end) = brackets.pop() {
                        buffer.push(tok.kind);
                        self.expect_char(end)?;
                    } else {
                        break;
                    }

                    wrote_newline = false;
                }
                ';' => {
                    if brackets.is_empty() {
                        break;
                    }

                    self.toks_mut().next();
                    buffer.push(';');
                    wrote_newline = false;
                }
                'u' | 'U' => {
                    if let Some(url) = self.try_parse_url()? {
                        buffer.push_str(&url);
                    } else {
                        buffer.push(tok.kind);
                        self.toks_mut().next();
                    }

                    wrote_newline = false;
                }
                c => {
                    if self.looking_at_identifier() {
                        buffer.push_str(&self.parse_identifier(false, false)?);
                    } else {
                        self.toks_mut().next();
                        buffer.push(c);
                    }

                    wrote_newline = false;
                }
            }
        }

        if let Some(last) = brackets.pop() {
            self.expect_char(last)?;
        }

        if !allow_empty && buffer.is_empty() {
            return Err(("Expected token.", self.toks().current_span()).into());
        }

        Ok(buffer)
    }

    fn try_parse_url(&mut self) -> SassResult<Option<String>> {
        let start = self.toks().cursor();

        if !self.scan_identifier("url", false)? {
            return Ok(None);
        }

        if !self.scan_char('(') {
            self.toks_mut().set_cursor(start);
            return Ok(None);
        }

        self.whitespace()?;

        // Match Ruby Sass's behavior: parse a raw URL() if possible, and if not
        // backtrack and re-parse as a function expression.
        let mut buffer = "url(".to_owned();

        while let Some(next) = self.toks().peek() {
            match next.kind {
                '\\' => {
                    buffer.push_str(&self.parse_escape(false)?);
                }
                '!' | '#' | '%' | '&' | '*'..='~' | '\u{80}'..=char::MAX => {
                    self.toks_mut().next();
                    buffer.push(next.kind);
                }
                ')' => {
                    self.toks_mut().next();
                    buffer.push(next.kind);

                    return Ok(Some(buffer));
                }
                ' ' | '\t' | '\n' | '\r' => {
                    self.whitespace_without_comments();

                    if !self.toks().next_char_is(')') {
                        break;
                    }
                }
                _ => break,
            }
        }

        self.toks_mut().set_cursor(start);
        Ok(None)
    }
}

impl<T: BaseParser + ?Sized> TokenParser for T {}
