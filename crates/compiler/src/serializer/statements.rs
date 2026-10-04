//! Statements: declarations, comments, imports and blocks.

use super::*;

/// The indentation of the least-indented non-empty line of `text` after the first: `None` if
/// `text` has no newline, and `Some(None)` if it has newlines but no such line.
pub(super) fn minimum_indentation(text: &str) -> Option<Option<usize>> {
    let (_, rest) = text.split_once('\n')?;
    Some(
        rest.split('\n')
            .filter_map(|line| {
                let indentation = line.len() - line.trim_start_matches([' ', '\t']).len();
                (indentation < line.len()).then_some(indentation)
            })
            .min(),
    )
}

impl<'a> Serializer<'a> {
    pub(super) fn write_style(&mut self, style: Style) -> SassResult<()> {
        if !self.options.is_compressed() {
            self.write_indentation();
        }

        self.buffer
            .extend_from_slice(style.property.resolve_ref().as_bytes());
        self.buffer.push(b':');

        if style.declared_as_custom_property {
            if let Value::String(text, QuoteKind::None) = &style.value.node {
                if self.options.is_compressed() {
                    self.write_folded_value(text);
                } else {
                    let column = self.map.look_up_pos(style.name_span.low()).position.column;
                    self.write_reindented_value(text, column);
                }
                return Ok(());
            }
        } else if !self.options.is_compressed() {
            self.buffer.push(b' ');
        }

        self.visit_value(&style.value.node, style.value.span)?;

        Ok(())
    }

    /// Writes a custom property's value with each newline, and the whitespace after it, as a space.
    pub(super) fn write_folded_value(&mut self, text: &str) {
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\n' {
                self.buffer
                    .extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
                continue;
            }
            self.buffer.push(b' ');
            while chars.peek().is_some_and(|c| c.is_ascii_whitespace()) {
                chars.next();
            }
        }
    }

    /// Writes a custom property's value reindented relative to the current indentation, where the
    /// property's name is at `name_column` in the source.
    pub(super) fn write_reindented_value(&mut self, text: &str, name_column: usize) {
        match minimum_indentation(text) {
            None => self.buffer.extend_from_slice(text.as_bytes()),
            Some(None) => {
                self.buffer.extend_from_slice(text.trim_end().as_bytes());
                self.buffer.push(b' ');
            }
            Some(Some(minimum)) => self.write_with_indent(text, minimum.min(name_column)),
        }
    }

    /// Writes `text`, replacing `minimum_indentation` with the current indentation on each
    /// non-empty line after the first, and trailing empty lines with a space.
    pub(super) fn write_with_indent(&mut self, text: &str, minimum_indentation: usize) {
        let (first, mut rest) = text.split_once('\n').unwrap_or((text, ""));
        self.buffer.extend_from_slice(first.as_bytes());

        loop {
            // The blank lines before the next one with text.
            let mut newlines = 1;
            let line = loop {
                let trimmed = rest.trim_start_matches([' ', '\t']);
                match trimmed.split_once('\n') {
                    _ if trimmed.is_empty() => {
                        // The whitespace could matter to a custom property.
                        self.buffer.push(b' ');
                        return;
                    }
                    Some(("", after)) => {
                        newlines += 1;
                        rest = after;
                    }
                    _ => break rest,
                }
            };

            for _ in 0..newlines {
                self.buffer.push(b'\n');
            }
            self.write_indentation();
            let (line, after) = match line.split_once('\n') {
                Some((line, after)) => (line, Some(after)),
                None => (line, None),
            };
            self.buffer
                .extend_from_slice(line.get(minimum_indentation..).unwrap_or("").as_bytes());
            match after {
                Some(after) => rest = after,
                None => return,
            }
        }
    }

    pub(super) fn write_import(
        &mut self,
        import: &str,
        modifiers: Option<String>,
    ) -> SassResult<()> {
        self.write_indentation();
        self.buffer.extend_from_slice(b"@import ");
        write!(&mut self.buffer, "{}", import)?;

        if let Some(modifiers) = modifiers {
            self.buffer.push(b' ');
            self.buffer.extend_from_slice(modifiers.as_bytes());
        }

        Ok(())
    }

    pub(super) fn write_comment(&mut self, comment: &str, span: Span) -> SassResult<()> {
        if self.options.is_compressed() && !comment.starts_with("/*!") {
            return Ok(());
        }

        self.write_indentation();
        let col = self.map.look_up_pos(span.low()).position.column;
        let mut lines = comment.lines();

        if let Some(line) = lines.next() {
            self.buffer.extend_from_slice(line.trim_start().as_bytes());
        }

        let lines = lines
            .map(|line| {
                let diff = (line.len() - line.trim_start().len()).saturating_sub(col);
                format!("{}{}", " ".repeat(diff), line.trim_start())
            })
            .collect::<Vec<String>>()
            .join("\n");

        if !lines.is_empty() {
            write!(&mut self.buffer, "\n{}", lines)?;
        }

        Ok(())
    }

    pub fn requires_semicolon(stmt: &CssStmt) -> bool {
        match stmt {
            CssStmt::Style(_) | CssStmt::Import(_, _) => true,
            CssStmt::UnknownAtRule(rule, _) => !rule.has_body,
            _ => false,
        }
    }

    pub(super) fn write_children(&mut self, mut children: Vec<CssStmt>) -> SassResult<()> {
        if self.options.is_compressed() {
            self.buffer.push(b'{');
        } else {
            self.buffer.extend_from_slice(b" {\n");
        }

        self.indentation += self.indent_width;

        let last = children.pop();

        for child in children {
            let needs_semicolon = Self::requires_semicolon(&child);
            let did_write = self.visit_stmt(child)?;

            if !did_write {
                continue;
            }

            if needs_semicolon {
                self.buffer.push(b';');
            }

            self.write_optional_newline();
        }

        if let Some(last) = last {
            let needs_semicolon = Self::requires_semicolon(&last);
            let did_write = self.visit_stmt(last)?;

            if did_write {
                if needs_semicolon && !self.options.is_compressed() {
                    self.buffer.push(b';');
                }

                self.write_optional_newline();
            }
        }

        self.indentation -= self.indent_width;

        if self.options.is_compressed() {
            self.buffer.push(b'}');
        } else {
            self.write_indentation();
            self.buffer.extend_from_slice(b"}");
        }

        Ok(())
    }

    pub(super) fn write_supports_rule(&mut self, supports_rule: SupportsRule) -> SassResult<()> {
        self.write_indentation();
        self.buffer.extend_from_slice(b"@supports");

        if !supports_rule.params.is_empty() {
            self.buffer.push(b' ');
            self.buffer
                .extend_from_slice(supports_rule.params.as_bytes());
        }

        self.write_children(supports_rule.body)?;

        Ok(())
    }

    /// Returns whether or not text was written
    pub(super) fn visit_stmt(&mut self, stmt: CssStmt) -> SassResult<bool> {
        if stmt.is_invisible() {
            return Ok(false);
        }

        match stmt {
            CssStmt::RuleSet { selector, body, .. } => {
                self.write_indentation();
                self.write_selector_list(&selector.as_selector_list());

                self.write_children(body)?;
            }
            CssStmt::Media(media_rule, ..) => {
                self.write_indentation();
                self.buffer.extend_from_slice(b"@media ");

                if let Some((last, rest)) = media_rule.query.split_last() {
                    for query in rest {
                        self.write_media_query(query);

                        self.buffer.push(b',');

                        self.write_optional_space();
                    }

                    self.write_media_query(last);
                }

                self.write_children(media_rule.body)?;
            }
            CssStmt::UnknownAtRule(unknown_at_rule, ..) => {
                self.write_indentation();
                self.buffer.push(b'@');
                self.buffer
                    .extend_from_slice(unknown_at_rule.name.as_bytes());

                if !unknown_at_rule.params.is_empty() {
                    write!(&mut self.buffer, " {}", unknown_at_rule.params)?;
                }

                if !unknown_at_rule.has_body {
                    debug_assert!(unknown_at_rule.body.is_empty());
                    return Ok(true);
                } else if unknown_at_rule.body.iter().all(CssStmt::is_invisible) {
                    self.buffer.extend_from_slice(b" {}");
                    return Ok(true);
                }

                self.write_children(unknown_at_rule.body)?;
            }
            CssStmt::Style(style) => self.write_style(style)?,
            CssStmt::Comment(comment, span) => self.write_comment(&comment, span)?,
            CssStmt::KeyframesRuleSet(keyframes_rule_set) => {
                self.write_indentation();
                // todo: i bet we can do something like write_with_separator to avoid extra allocation
                let selector = keyframes_rule_set
                    .selector
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<String>>()
                    .join(", ");

                self.buffer.extend_from_slice(selector.as_bytes());

                self.write_children(keyframes_rule_set.body)?;
            }
            CssStmt::Import(import, modifier) => self.write_import(&import, modifier)?,
            CssStmt::Supports(supports_rule, _) => self.write_supports_rule(supports_rule)?,
        }

        Ok(true)
    }
}
