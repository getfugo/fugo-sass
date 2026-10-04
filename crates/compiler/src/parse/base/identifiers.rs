//! Identifiers, escapes and keywords.

use super::*;

/// Identifiers, escapes and keywords.
pub(crate) trait IdentifierParser: BaseParser {
    fn parse_identifier(
        &mut self,
        // default=false
        normalize: bool,
        // default=false
        unit: bool,
    ) -> SassResult<String> {
        let mut text = String::new();

        if self.scan_char('-') {
            text.push('-');

            if self.scan_char('-') {
                text.push('-');
                self.parse_identifier_body(&mut text, normalize, unit)?;
                return Ok(text);
            }
        }

        match self.toks().peek() {
            Some(Token { kind: '_', .. }) if normalize => {
                self.toks_mut().next();
                text.push('-');
            }
            Some(Token { kind, .. }) if is_name_start(kind) => {
                self.toks_mut().next();
                text.push(kind);
            }
            Some(Token { kind: '\\', .. }) => {
                text.push_str(&self.parse_escape(true)?);
            }
            Some(..) | None => {
                return Err(("Expected identifier.", self.toks().current_span()).into());
            }
        }

        self.parse_identifier_body(&mut text, normalize, unit)?;

        Ok(text)
    }

    fn parse_identifier_body(
        &mut self,
        buffer: &mut String,
        normalize: bool,
        unit: bool,
    ) -> SassResult<()> {
        while let Some(tok) = self.toks().peek() {
            if unit && tok.kind == '-' {
                // Disallow `-` followed by a dot or a digit digit in units.
                let second = match self.toks().peek_n(1) {
                    Some(v) => v,
                    None => break,
                };

                if second.kind == '.' || second.kind.is_ascii_digit() {
                    break;
                }

                self.toks_mut().next();
                buffer.push('-');
            } else if normalize && tok.kind == '_' {
                buffer.push('-');
                self.toks_mut().next();
            } else if is_name(tok.kind) {
                self.toks_mut().next();
                buffer.push(tok.kind);
            } else if tok.kind == '\\' {
                buffer.push_str(&self.parse_escape(false)?);
            } else {
                break;
            }
        }

        Ok(())
    }

    fn parse_escape(&mut self, identifier_start: bool) -> SassResult<String> {
        let start = self.toks().cursor();
        self.expect_char('\\')?;
        let mut value = 0;
        let first = match self.toks().peek() {
            Some(t) => t,
            None => return Err(("Expected expression.", self.toks().current_span()).into()),
        };
        if first.kind == '\n' {
            return Err(("Expected escape sequence.", self.toks().current_span()).into());
        } else if first.kind.is_ascii_hexdigit() {
            for _ in 0..6 {
                let next = match self.toks().peek() {
                    Some(t) => t,
                    None => break,
                };
                if !next.kind.is_ascii_hexdigit() {
                    break;
                }
                value *= 16;
                value += as_hex(next.kind);
                self.toks_mut().next();
            }
            if matches!(
                self.toks().peek(),
                Some(Token { kind: ' ', .. })
                    | Some(Token { kind: '\n', .. })
                    | Some(Token { kind: '\t', .. })
            ) {
                self.toks_mut().next();
            }
        } else {
            value = first.kind as u32;
            self.toks_mut().next();
        }

        let c = std::char::from_u32(value)
            .ok_or_else(|| ("Invalid Unicode code point.", self.toks().span_from(start)))?;
        if (identifier_start && is_name_start(c) && !c.is_ascii_digit())
            || (!identifier_start && is_name(c))
        {
            Ok(c.to_string())
        } else if value <= 0x1F || value == 0x7F || (identifier_start && c.is_ascii_digit()) {
            let mut buf = String::with_capacity(4);
            buf.push('\\');
            if value > 0xF {
                buf.push(hex_char_for(value >> 4));
            }
            buf.push(hex_char_for(value & 0xF));
            buf.push(' ');
            Ok(buf)
        } else {
            Ok(format!("\\{}", c))
        }
    }

    fn consume_escaped_char(&mut self) -> SassResult<char> {
        self.expect_char('\\')?;

        match self.toks().peek() {
            None => Ok('\u{FFFD}'),
            Some(Token {
                kind: '\n' | '\r', ..
            }) => Err(("Expected escape sequence.", self.toks().current_span()).into()),
            Some(Token { kind, .. }) if kind.is_ascii_hexdigit() => {
                let mut value = 0;
                for _ in 0..6 {
                    let next = match self.toks().peek() {
                        Some(c) => c,
                        None => break,
                    };
                    if !next.kind.is_ascii_hexdigit() {
                        break;
                    }
                    self.toks_mut().next();
                    value = (value << 4) + as_hex(next.kind);
                }

                if self.toks().peek().is_some()
                    && self.toks().peek().unwrap().kind.is_ascii_whitespace()
                {
                    self.toks_mut().next();
                }

                if value == 0 || (0xD800..=0xDFFF).contains(&value) || value >= 0x0010_FFFF {
                    Ok('\u{FFFD}')
                } else {
                    Ok(char::from_u32(value).unwrap())
                }
            }
            Some(Token { kind, .. }) => {
                self.toks_mut().next();
                Ok(kind)
            }
        }
    }

    /// Returns whether the scanner is immediately before a plain CSS identifier.
    ///
    /// This is based on [the CSS algorithm][], but it assumes all backslashes
    /// start escapes.
    ///
    /// [the CSS algorithm]: https://drafts.csswg.org/css-syntax-3/#would-start-an-identifier
    fn looking_at_identifier(&self) -> bool {
        match self.toks().peek() {
            Some(Token { kind, .. }) if is_name_start(kind) || kind == '\\' => return true,
            Some(Token { kind: '-', .. }) => {}
            Some(..) | None => return false,
        }

        match self.toks().peek_n(1) {
            Some(Token { kind, .. }) if is_name_start(kind) || kind == '-' || kind == '\\' => true,
            Some(..) | None => false,
        }
    }

    /// Peeks to see if the `ident` is at the current position. If it is,
    /// consume the identifier
    fn scan_identifier(
        &mut self,
        ident: &'static str,
        // default=false
        case_sensitive: bool,
    ) -> SassResult<bool> {
        if !self.looking_at_identifier() {
            return Ok(false);
        }

        let start = self.toks().cursor();

        if self.consume_identifier(ident, case_sensitive)? && !self.looking_at_identifier_body() {
            Ok(true)
        } else {
            self.toks_mut().set_cursor(start);
            Ok(false)
        }
    }

    fn consume_identifier(&mut self, ident: &str, case_sensitive: bool) -> SassResult<bool> {
        for c in ident.chars() {
            if !self.scan_ident_char(c, case_sensitive)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn scan_ident_char(&mut self, c: char, case_sensitive: bool) -> SassResult<bool> {
        let matches = |actual: char| {
            if case_sensitive {
                actual == c
            } else {
                actual.eq_ignore_ascii_case(&c)
            }
        };

        Ok(match self.toks().peek() {
            Some(Token { kind, .. }) if matches(kind) => {
                self.toks_mut().next();
                true
            }
            Some(Token { kind: '\\', .. }) => {
                let start = self.toks().cursor();
                if matches(self.consume_escaped_char()?) {
                    return Ok(true);
                }
                self.toks_mut().set_cursor(start);
                false
            }
            Some(..) | None => false,
        })
    }

    fn expect_ident_char(&mut self, c: char, case_sensitive: bool) -> SassResult<()> {
        if self.scan_ident_char(c, case_sensitive)? {
            return Ok(());
        }

        Err((format!("Expected \"{}\".", c), self.toks().current_span()).into())
    }

    fn looking_at_identifier_body(&mut self) -> bool {
        matches!(self.toks().peek(), Some(t) if is_name(t.kind) || t.kind == '\\')
    }

    fn parse_variable_name(&mut self) -> SassResult<String> {
        self.expect_char('$')?;
        self.parse_identifier(true, false)
    }

    fn expect_identifier(&mut self, ident: &str, case_sensitive: bool) -> SassResult<()> {
        let start = self.toks().cursor();

        for c in ident.chars() {
            if !self.scan_ident_char(c, case_sensitive)? {
                return Err((
                    format!("Expected \"{}\".", ident),
                    self.toks_mut().span_from(start),
                )
                    .into());
            }
        }

        if !self.looking_at_identifier_body() {
            return Ok(());
        }

        Err((
            format!("Expected \"{}\".", ident),
            self.toks_mut().span_from(start),
        )
            .into())
    }
}

impl<T: BaseParser + ?Sized> IdentifierParser for T {}
