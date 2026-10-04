//! Indentation: reading it, checking that it is consistent, and the statements it nests.

use super::*;

impl<'a> SassParser<'a> {
    pub fn new(
        toks: Lexer,
        options: &'a Options<'a>,
        empty_span: Span,
        file_name: &'a Path,
    ) -> Self {
        let mut flags = ContextFlags::empty();

        flags.set(ContextFlags::IS_USE_ALLOWED, true);

        SassParser {
            toks,
            path: file_name,
            empty_span,
            flags,
            options,
            current_indentation: 0,
            next_indentation: None,
            next_indentation_end: None,
            spaces: None,
        }
    }

    pub(super) fn peek_indentation(&mut self) -> SassResult<usize> {
        if let Some(next) = self.next_indentation {
            return Ok(next);
        }

        if self.toks.peek().is_none() {
            self.next_indentation = Some(0);
            self.next_indentation_end = Some(self.toks.cursor());
            return Ok(0);
        }

        let start = self.toks.cursor();

        if !self.scan_char('\n') {
            return Err(("Expected newline.", self.toks.current_span()).into());
        }

        let mut contains_tab;
        let mut contains_space;
        let mut next_indentation;

        loop {
            contains_tab = false;
            contains_space = false;
            next_indentation = 0;

            while let Some(next) = self.toks.peek() {
                match next.kind {
                    ' ' => contains_space = true,
                    '\t' => contains_tab = true,
                    _ => break,
                }

                next_indentation += 1;
                self.toks.next();
            }

            if self.toks.peek().is_none() {
                self.next_indentation = Some(0);
                self.next_indentation_end = Some(self.toks.cursor());
                self.toks.set_cursor(start);
                return Ok(0);
            }

            if !self.scan_char('\n') {
                break;
            }
        }

        self.check_indentation_consistency(contains_tab, contains_space, start)?;

        self.next_indentation = Some(next_indentation);

        if next_indentation > 0 {
            self.spaces.get_or_insert(contains_space);
        }

        self.next_indentation_end = Some(self.toks.cursor());
        self.toks.set_cursor(start);

        Ok(next_indentation)
    }

    pub(super) fn check_indentation_consistency(
        &mut self,
        contains_tab: bool,
        contains_space: bool,
        start: usize,
    ) -> SassResult<()> {
        // NOTE: error message spans here start from the beginning of the line
        if contains_tab {
            if contains_space {
                return Err((
                    "Tabs and spaces may not be mixed.",
                    self.toks.span_from(start),
                )
                    .into());
            } else if self.spaces == Some(true) {
                return Err(("Expected spaces, was tabs.", self.toks.span_from(start)).into());
            }
        } else if contains_space && self.spaces == Some(false) {
            return Err(("Expected tabs, was spaces.", self.toks.span_from(start)).into());
        }

        Ok(())
    }

    /// Consumes a newline. `trailing_semicolon` is whether a semicolon ended the statement, for the
    /// error.
    pub(super) fn expect_newline(&mut self, trailing_semicolon: bool) -> SassResult<()> {
        match self.toks.peek() {
            Some(Token { kind: '\r', .. }) => {
                self.toks.next();
                self.scan_char('\n');
                Ok(())
            }
            Some(Token { kind: '\n', .. }) => {
                self.toks.next();
                Ok(())
            }
            _ => Err((
                if trailing_semicolon {
                    "multiple statements on one line are not supported in the indented syntax."
                } else {
                    "expected newline."
                },
                self.toks.current_span(),
            )
                .into()),
        }
    }

    /// Consumes a semicolon that ends a statement, as dart-sass 1.105.1 allows, and the whitespace
    /// after it.
    pub(super) fn try_trailing_semicolon(&mut self) -> SassResult<bool> {
        if self.scan_char(';') {
            self.whitespace()?;
            return Ok(true);
        }
        Ok(false)
    }

    pub(super) fn read_indentation(&mut self) -> SassResult<usize> {
        self.current_indentation = match self.next_indentation {
            Some(indent) => indent,
            None => {
                let indent = self.peek_indentation()?;
                self.next_indentation = Some(indent);
                indent
            }
        };

        self.toks.set_cursor(self.next_indentation_end.unwrap());
        self.next_indentation = None;
        self.next_indentation_end = None;

        Ok(self.current_indentation)
    }

    pub(super) fn while_indented_lower(
        &mut self,
        mut body: impl FnMut(&mut Self) -> SassResult<()>,
    ) -> SassResult<()> {
        let parent_indentation = self.current_indentation;
        let mut child_indentation = None;

        while self.peek_indentation()? > parent_indentation {
            let indentation = self.read_indentation()?;
            let child_indent = *child_indentation.get_or_insert(indentation);

            if child_indent != indentation {
                return Err((
                    format!(
                        "Inconsistent indentation, expected {child_indent} spaces.",
                        child_indent = child_indent
                    ),
                    self.toks.current_span(),
                )
                    .into());
            }

            body(self)?;
        }

        Ok(())
    }

    pub(super) fn parse_child(
        &mut self,
        child: impl FnOnce(&mut Self) -> SassResult<Option<AstStmt>>,
    ) -> SassResult<Option<AstStmt>> {
        Ok(Some(match self.toks.peek() {
            Some(Token {
                kind: '\n' | '\r', ..
            }) => return Ok(None),
            Some(Token { kind: '$', .. }) => AstStmt::VariableDecl(
                self.parse_variable_declaration_without_namespace(None, None)?,
            ),
            Some(Token { kind: '/', .. }) => match self.toks.peek_n(1) {
                Some(Token { kind: '/', .. }) => self.parse_silent_comment()?,
                Some(Token { kind: '*', .. }) => AstStmt::LoudComment(self.parse_loud_comment()?),
                _ => return child(self),
            },
            _ => return child(self),
        }))
    }

    pub(super) fn looking_at_double_newline(&mut self) -> bool {
        match self.toks.peek() {
            // todo: is this branch reachable
            Some(Token { kind: '\r', .. }) => match self.toks.peek_n(1) {
                Some(Token { kind: '\n', .. }) => {
                    matches!(self.toks.peek_n(2), Some(Token { kind: '\n', .. }))
                }
                Some(Token { kind: '\r', .. }) => true,
                _ => false,
            },
            Some(Token { kind: '\n', .. }) => matches!(
                self.toks.peek_n(1),
                Some(Token {
                    kind: '\n' | '\r',
                    ..
                })
            ),
            _ => false,
        }
    }
}
