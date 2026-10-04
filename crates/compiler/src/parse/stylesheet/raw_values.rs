//! Values kept mostly as written: declaration values, selectors and string tokens.

use super::*;

/// Values kept mostly as written: declaration values, selectors and string tokens.
pub(crate) trait RawValueParser<'a>: StylesheetParser<'a> {
    /// Parses a quoted string as written, quotes and escapes included (dart-sass's
    /// `interpolatedStringToken`).
    fn parse_interpolated_string_token(&mut self) -> SassResult<Interpolation> {
        let quote = match self.toks_mut().next() {
            Some(Token {
                kind: q @ ('"' | '\''),
                ..
            }) => q,
            _ => return Err(("Expected string.", self.toks().current_span()).into()),
        };

        let mut buffer = Interpolation::new();
        buffer.add_char(quote);
        loop {
            match self.toks().peek() {
                Some(Token { kind, .. }) if kind == quote => {
                    self.toks_mut().next();
                    buffer.add_char(quote);
                    break;
                }
                None
                | Some(Token {
                    kind: '\n' | '\r' | '\u{c}',
                    ..
                }) => return Err((format!("Expected {quote}."), self.toks().current_span()).into()),
                Some(Token { kind: '\\', .. }) => match self.toks().peek_n(1) {
                    Some(Token {
                        kind: second @ ('\n' | '\r' | '\u{c}'),
                        ..
                    }) => {
                        self.toks_mut().next();
                        self.toks_mut().next();
                        buffer.add_char('\\');
                        buffer.add_char(second);
                        if second == '\r' && self.scan_char('\n') {
                            buffer.add_char('\n');
                        }
                    }
                    _ => {
                        let escape = self.fallible_raw_text(Self::consume_escaped_char)?;
                        buffer.add_string(escape);
                    }
                },
                Some(Token { kind: '#', .. })
                    if matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })) =>
                {
                    buffer.add_interpolation(self.parse_single_interpolation()?);
                }
                Some(Token { kind, .. }) => {
                    self.toks_mut().next();
                    buffer.add_char(kind);
                }
            }
        }

        Ok(buffer)
    }

    fn parse_interpolated_declaration_value(
        &mut self,
        // default=false
        allow_semicolon: bool,
        // default=false
        allow_empty: bool,
        // default=true
        allow_colon: bool,
        // default=false: whether newlines are whitespace in the indented syntax
        consume_newlines: bool,
    ) -> SassResult<Interpolation> {
        let mut buffer = Interpolation::new();

        let mut brackets = Vec::new();
        let mut wrote_newline = false;

        while let Some(tok) = self.toks().peek() {
            match tok.kind {
                '\\' => {
                    buffer.add_string(self.parse_escape(true)?);
                    wrote_newline = false;
                }
                '"' | '\'' => {
                    buffer.add_interpolation(self.parse_interpolated_string_token()?);
                    wrote_newline = false;
                }
                '/' => {
                    if matches!(self.toks().peek_n(1), Some(Token { kind: '*', .. })) {
                        let comment = self.fallible_raw_text(Self::skip_loud_comment)?;
                        buffer.add_string(comment);
                    } else {
                        self.toks_mut().next();
                        buffer.add_char(tok.kind);
                    }

                    wrote_newline = false;
                }
                '#' => {
                    if matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })) {
                        // Add a full interpolated identifier to handle cases like
                        // "#{...}--1", since "--1" isn't a valid identifier on its own.
                        buffer.add_interpolation(self.parse_interpolated_identifier()?);
                    } else {
                        self.toks_mut().next();
                        buffer.add_char(tok.kind);
                    }

                    wrote_newline = false;
                }
                ' ' | '\t' => {
                    if wrote_newline
                        || !matches!(
                            self.toks().peek_n(1),
                            Some(Token {
                                kind: ' ' | '\r' | '\t' | '\n',
                                ..
                            })
                        )
                    {
                        self.toks_mut().next();
                        buffer.add_char(tok.kind);
                    } else {
                        self.toks_mut().next();
                    }
                }
                '\n' | '\r' => {
                    if self.is_indented() && !consume_newlines && brackets.is_empty() {
                        break;
                    }
                    if !matches!(
                        self.toks().peek_n_backwards(1),
                        Some(Token {
                            kind: '\r' | '\n',
                            ..
                        })
                    ) {
                        buffer.add_char('\n');
                    }
                    self.toks_mut().next();
                    wrote_newline = true;
                }
                '(' | '{' | '[' => {
                    self.toks_mut().next();
                    buffer.add_char(tok.kind);
                    brackets.push(opposite_bracket(tok.kind));
                    wrote_newline = false;
                }
                ')' | '}' | ']' => {
                    if brackets.is_empty() {
                        break;
                    }
                    buffer.add_char(tok.kind);
                    self.expect_char(brackets.pop().unwrap())?;
                    wrote_newline = false;
                }
                ';' => {
                    if !allow_semicolon && brackets.is_empty() {
                        break;
                    }
                    buffer.add_char(tok.kind);
                    self.toks_mut().next();
                    wrote_newline = false;
                }
                ':' => {
                    if !allow_colon && brackets.is_empty() {
                        break;
                    }
                    buffer.add_char(tok.kind);
                    self.toks_mut().next();
                    wrote_newline = false;
                }
                'u' | 'U' => {
                    let before_url = self.toks().cursor();

                    if !self.scan_identifier("url", false)? {
                        buffer.add_char(tok.kind);
                        self.toks_mut().next();
                        wrote_newline = false;
                        continue;
                    }

                    match self.try_url_contents(None)? {
                        Some(contents) => {
                            buffer.add_interpolation(contents);
                        }
                        None => {
                            self.toks_mut().set_cursor(before_url);
                            buffer.add_char(tok.kind);
                            self.toks_mut().next();
                        }
                    }

                    wrote_newline = false;
                }
                _ => {
                    if self.looking_at_identifier() {
                        buffer.add_string(self.parse_identifier(false, false)?);
                    } else {
                        buffer.add_char(tok.kind);
                        self.toks_mut().next();
                    }
                    wrote_newline = false;
                }
            }
        }

        if let Some(&last) = brackets.last() {
            self.expect_char(last)?;
        }

        if !allow_empty && buffer.contents.is_empty() {
            return Err(("Expected token.", self.toks().current_span()).into());
        }

        Ok(buffer)
    }

    fn almost_any_value(
        &mut self,
        // default=false
        omit_comments: bool,
    ) -> SassResult<Interpolation> {
        let mut buffer = Interpolation::new();
        let mut brackets = Vec::new();

        while let Some(tok) = self.toks().peek() {
            match tok.kind {
                '\\' => {
                    // Write a literal backslash because this text will be re-parsed.
                    buffer.add_char(tok.kind);
                    self.toks_mut().next();
                    match self.toks_mut().next() {
                        Some(tok) => buffer.add_char(tok.kind),
                        None => {
                            return Err(("expected more input.", self.toks().current_span()).into());
                        }
                    }
                }
                '"' | '\'' => {
                    buffer.add_interpolation(self.parse_interpolated_string_token()?);
                }
                '/' => {
                    let comment_start = self.toks().cursor();
                    if self.scan_comment()? {
                        if !omit_comments {
                            buffer.add_string(self.toks().raw_text(comment_start));
                        }
                    } else {
                        buffer.add_char(self.toks_mut().next().unwrap().kind);
                    }
                }
                '#' => {
                    if matches!(self.toks().peek_n(1), Some(Token { kind: '{', .. })) {
                        // Add a full interpolated identifier to handle cases like
                        // "#{...}--1", since "--1" isn't a valid identifier on its own.
                        buffer.add_interpolation(self.parse_interpolated_identifier()?);
                    } else {
                        self.toks_mut().next();
                        buffer.add_char(tok.kind);
                    }
                }
                '\r' | '\n' => {
                    if self.is_indented() && brackets.is_empty() {
                        break;
                    }
                    buffer.add_char(self.toks_mut().next().unwrap().kind);
                }
                '!' | ';' | '{' | '}' => break,
                'u' | 'U' => {
                    let before_url = self.toks().cursor();
                    let identifier = self.parse_identifier(false, false)?;
                    // `url-prefix()` isn't standard CSS, but the old `@document` rule had it.
                    if identifier != "url" && identifier != "url-prefix" {
                        buffer.add_string(identifier);
                        continue;
                    }

                    match self.try_url_contents(Some(&identifier))? {
                        Some(contents) => buffer.add_interpolation(contents),
                        None => {
                            self.toks_mut().set_cursor(before_url);
                            self.toks_mut().next();
                            buffer.add_char(tok.kind);
                        }
                    }
                }
                '(' | '[' => {
                    self.toks_mut().next();
                    buffer.add_char(tok.kind);
                    brackets.push(opposite_bracket(tok.kind));
                }
                ')' | ']' => {
                    let Some(bracket) = brackets.pop() else {
                        return Err((
                            format!("Unexpected \"{}\".", tok.kind),
                            self.toks().current_span(),
                        )
                            .into());
                    };
                    self.expect_char(bracket)?;
                    buffer.add_char(bracket);
                }
                _ => {
                    if self.looking_at_identifier() {
                        buffer.add_string(self.parse_identifier(false, false)?);
                    } else {
                        buffer.add_char(self.toks_mut().next().unwrap().kind);
                    }
                }
            }
        }

        Ok(buffer)
    }
}

impl<'a, T: StylesheetParser<'a>> RawValueParser<'a> for T {}
