use crate::{
    Token,
    error::SassResult,
    lexer::Lexer,
    utils::{as_hex, hex_char_for, is_name, is_name_start, opposite_bracket},
};

mod identifiers;
mod tokens;

pub(crate) use identifiers::IdentifierParser;
pub(crate) use tokens::TokenParser;

pub(crate) trait BaseParser {
    fn toks(&self) -> &Lexer;
    fn toks_mut(&mut self) -> &mut Lexer;

    fn whitespace_without_comments(&mut self) {
        while matches!(
            self.toks().peek(),
            Some(Token {
                kind: ' ' | '\t' | '\n',
                ..
            })
        ) {
            self.toks_mut().next();
        }
    }

    fn whitespace(&mut self) -> SassResult<()> {
        loop {
            self.whitespace_without_comments();

            if !self.scan_comment()? {
                break;
            }
        }

        Ok(())
    }

    /// Like `whitespace()`, but newlines are whitespace in the indented syntax too: for positions
    /// where a statement can't end (dart-sass's `whitespace(consumeNewlines: true)`, where
    /// `whitespace()` is `consumeNewlines: false`).
    fn whitespace_with_newlines(&mut self) -> SassResult<()> {
        loop {
            self.whitespace_without_comments_with_newlines();

            if !self.scan_comment()? {
                break;
            }
        }

        Ok(())
    }

    /// Like `whitespace_without_comments()`, but newlines are whitespace in the indented syntax too.
    fn whitespace_without_comments_with_newlines(&mut self) {
        while matches!(
            self.toks().peek(),
            Some(Token {
                kind: ' ' | '\t' | '\n',
                ..
            })
        ) {
            self.toks_mut().next();
        }
    }

    /// `whitespace_with_newlines()` if `consume_newlines`, else `whitespace()`.
    fn whitespace_newlines_if(&mut self, consume_newlines: bool) -> SassResult<()> {
        if consume_newlines {
            self.whitespace_with_newlines()
        } else {
            self.whitespace()
        }
    }

    fn scan_comment(&mut self) -> SassResult<bool> {
        if !matches!(self.toks().peek(), Some(Token { kind: '/', .. })) {
            return Ok(false);
        }

        Ok(match self.toks().peek_n(1) {
            Some(Token { kind: '/', .. }) => {
                self.skip_silent_comment()?;
                true
            }
            Some(Token { kind: '*', .. }) => {
                self.skip_loud_comment()?;
                true
            }
            _ => false,
        })
    }

    fn skip_silent_comment(&mut self) -> SassResult<()> {
        debug_assert!(self.next_matches("//"));
        self.toks_mut().next();
        self.toks_mut().next();
        while self.toks().peek().is_some() && !self.toks().next_char_is('\n') {
            self.toks_mut().next();
        }
        Ok(())
    }

    fn next_matches(&mut self, s: &str) -> bool {
        for (idx, c) in s.chars().enumerate() {
            match self.toks().peek_n(idx) {
                Some(Token { kind, .. }) if kind == c => {}
                _ => return false,
            }
        }

        true
    }

    fn skip_loud_comment(&mut self) -> SassResult<()> {
        debug_assert!(self.next_matches("/*"));
        self.toks_mut().next();
        self.toks_mut().next();

        while let Some(next) = self.toks_mut().next() {
            if next.kind != '*' {
                continue;
            }

            while self.scan_char('*') {}

            if self.scan_char('/') {
                return Ok(());
            }
        }

        Err(("expected more input.", self.toks().current_span()).into())
    }

    fn scan_char(&mut self, c: char) -> bool {
        if let Some(Token { kind, .. }) = self.toks().peek()
            && kind == c
        {
            self.toks_mut().next();
            return true;
        }

        false
    }

    fn scan(&mut self, s: &str) -> bool {
        let start = self.toks().cursor();
        for c in s.chars() {
            if !self.scan_char(c) {
                self.toks_mut().set_cursor(start);
                return false;
            }
        }

        true
    }

    fn expect_whitespace(&mut self) -> SassResult<()> {
        self.expect_whitespace_newlines_if(false)
    }

    /// `expect_whitespace()`, with newlines as whitespace in the indented syntax if
    /// `consume_newlines`.
    fn expect_whitespace_newlines_if(&mut self, consume_newlines: bool) -> SassResult<()> {
        if !matches!(
            self.toks().peek(),
            Some(Token {
                kind: ' ' | '\t' | '\n' | '\r',
                ..
            })
        ) && !self.scan_comment()?
        {
            return Err(("Expected whitespace.", self.toks().current_span()).into());
        }

        self.whitespace_newlines_if(consume_newlines)
    }

    fn expect_char(&mut self, c: char) -> SassResult<()> {
        match self.toks().peek() {
            Some(tok) if tok.kind == c => {
                self.toks_mut().next();
                Ok(())
            }
            Some(..) | None => {
                Err((format!("expected \"{}\".", c), self.toks().current_span()).into())
            }
        }
    }

    fn expect_char_with_message(&mut self, c: char, msg: &'static str) -> SassResult<()> {
        match self.toks().peek() {
            Some(tok) if tok.kind == c => {
                self.toks_mut().next();
                Ok(())
            }
            Some(..) | None => Err((format!("expected {}.", msg), self.toks().prev_span()).into()),
        }
    }

    fn raw_text<T>(&mut self, func: impl Fn(&mut Self) -> T) -> String {
        let start = self.toks().cursor();
        func(self);
        self.toks().raw_text(start)
    }

    fn fallible_raw_text<T>(
        &mut self,
        func: impl Fn(&mut Self) -> SassResult<T>,
    ) -> SassResult<String> {
        let start = self.toks().cursor();
        func(self)?;
        Ok(self.toks().raw_text(start))
    }

    // todo: not real impl
    fn expect_done(&mut self) -> SassResult<()> {
        debug_assert!(self.toks().peek().is_none());

        Ok(())
    }

    fn spaces(&mut self) {
        while self.toks().next_char_is(' ') || self.toks().next_char_is('\t') {
            self.toks_mut().next();
        }
    }
}
