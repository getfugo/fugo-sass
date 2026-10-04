//! Simple selectors: attributes, classes, IDs, pseudo-selectors, `&`, placeholders, types and `an+b`.

use super::*;

impl SelectorParser {
    pub(super) fn parse_attribute_selector(&mut self) -> SassResult<SimpleSelector> {
        self.toks.next();
        Ok(SimpleSelector::Attribute(Box::new(Attribute::from_tokens(
            self,
        )?)))
    }

    pub(super) fn parse_class_selector(&mut self) -> SassResult<SimpleSelector> {
        self.toks.next();
        Ok(SimpleSelector::Class(self.parse_identifier(false, false)?))
    }

    pub(super) fn parse_id_selector(&mut self) -> SassResult<SimpleSelector> {
        self.toks.next();
        Ok(SimpleSelector::Id(self.parse_identifier(false, false)?))
    }

    pub(super) fn parse_pseudo_selector(&mut self) -> SassResult<SimpleSelector> {
        self.toks.next();
        let element = self.scan_char(':');
        let name = self.parse_identifier(false, false)?;

        match self.toks.peek() {
            Some(Token { kind: '(', .. }) => self.toks.next(),
            _ => {
                return Ok(SimpleSelector::Pseudo(Pseudo {
                    is_class: !element && !is_fake_pseudo_element(&name),
                    name,
                    selector: None,
                    is_syntactic_class: !element,
                    argument: None,
                    span: self.span,
                }));
            }
        };

        self.whitespace()?;

        let unvendored = unvendor(&name);

        let mut argument: Option<Box<str>> = None;
        let mut selector: Option<Box<SelectorList>> = None;

        if element {
            // todo: lowercase?
            if SELECTOR_PSEUDO_ELEMENTS.contains(&unvendored) {
                selector = Some(Box::new(self.parse_selector_list()?));
                self.whitespace()?;
            } else {
                argument = Some(self.declaration_value(true)?.into_boxed_str());
            }

            self.expect_char(')')?;
        } else if SELECTOR_PSEUDO_CLASSES.contains(&unvendored) {
            selector = Some(Box::new(self.parse_selector_list()?));
            self.whitespace()?;
            self.expect_char(')')?;
        } else if unvendored == "nth-child" || unvendored == "nth-last-child" {
            let mut this_arg = self.parse_a_n_plus_b()?;
            self.whitespace()?;

            let last_was_whitespace = matches!(
                self.toks.peek_n_backwards(1),
                Some(Token {
                    kind: ' ' | '\t' | '\n' | '\r',
                    ..
                })
            );
            if last_was_whitespace && !matches!(self.toks.peek(), Some(Token { kind: ')', .. })) {
                self.expect_identifier("of", false)?;
                this_arg.push_str(" of");
                self.whitespace()?;
                selector = Some(Box::new(self.parse_selector_list()?));
            }

            self.expect_char(')')?;
            argument = Some(this_arg.into_boxed_str());
        } else {
            argument = Some(
                self.declaration_value(true)?
                    .trim_end()
                    .to_owned()
                    .into_boxed_str(),
            );

            self.expect_char(')')?;
        }

        Ok(SimpleSelector::Pseudo(Pseudo {
            is_class: !element && !is_fake_pseudo_element(&name),
            name,
            selector,
            is_syntactic_class: !element,
            argument,
            span: self.span,
        }))
    }

    pub(super) fn parse_parent_selector(&mut self) -> SassResult<SimpleSelector> {
        self.toks.next();
        let suffix = if self.looking_at_identifier_body() {
            let mut buffer = String::new();
            self.parse_identifier_body(&mut buffer, false, false)?;
            Some(buffer)
        } else {
            None
        };
        if self.plain_css && suffix.is_some() {
            return Err((
                "Parent selectors can't have suffixes in plain CSS.",
                self.span,
            )
                .into());
        }
        Ok(SimpleSelector::Parent(suffix))
    }

    pub(super) fn parse_placeholder_selector(&mut self) -> SassResult<SimpleSelector> {
        self.toks.next();
        Ok(SimpleSelector::Placeholder(
            self.parse_identifier(false, false)?,
        ))
    }

    /// Consumes a type selector or a universal selector.
    ///
    /// These are combined because either one could start with `*`.
    pub(super) fn parse_type_or_universal_selector(&mut self) -> SassResult<SimpleSelector> {
        match self.toks.peek() {
            Some(Token { kind: '*', .. }) => {
                self.toks.next();
                if let Some(Token { kind: '|', .. }) = self.toks.peek() {
                    self.toks.next();
                    if let Some(Token { kind: '*', .. }) = self.toks.peek() {
                        self.toks.next();
                        return Ok(SimpleSelector::Universal(Namespace::Asterisk));
                    }

                    return Ok(SimpleSelector::Type(QualifiedName {
                        ident: self.parse_identifier(false, false)?,
                        namespace: Namespace::Asterisk,
                    }));
                }

                return Ok(SimpleSelector::Universal(Namespace::None));
            }
            Some(Token { kind: '|', .. }) => {
                self.toks.next();
                match self.toks.peek() {
                    Some(Token { kind: '*', .. }) => {
                        self.toks.next();
                        return Ok(SimpleSelector::Universal(Namespace::Empty));
                    }
                    _ => {
                        return Ok(SimpleSelector::Type(QualifiedName {
                            ident: self.parse_identifier(false, false)?,
                            namespace: Namespace::Empty,
                        }));
                    }
                }
            }
            _ => {}
        }

        let name_or_namespace = self.parse_identifier(false, false)?;

        Ok(match self.toks.peek() {
            Some(Token { kind: '|', .. }) => {
                self.toks.next();
                if let Some(Token { kind: '*', .. }) = self.toks.peek() {
                    self.toks.next();
                    SimpleSelector::Universal(Namespace::Other(name_or_namespace.into_boxed_str()))
                } else {
                    SimpleSelector::Type(QualifiedName {
                        ident: self.parse_identifier(false, false)?,
                        namespace: Namespace::Other(name_or_namespace.into_boxed_str()),
                    })
                }
            }
            Some(..) | None => SimpleSelector::Type(QualifiedName {
                ident: name_or_namespace,
                namespace: Namespace::None,
            }),
        })
    }

    /// Consumes an [`An+B` production][An+B] and returns its text.
    ///
    /// [An+B]: https://drafts.csswg.org/css-syntax-3/#anb-microsyntax
    pub(super) fn parse_a_n_plus_b(&mut self) -> SassResult<String> {
        let mut buf = String::new();

        match self.toks.peek() {
            Some(Token { kind: 'e', .. }) | Some(Token { kind: 'E', .. }) => {
                self.expect_identifier("even", false)?;
                return Ok("even".to_owned());
            }
            Some(Token { kind: 'o', .. }) | Some(Token { kind: 'O', .. }) => {
                self.expect_identifier("odd", false)?;
                return Ok("odd".to_owned());
            }
            Some(t @ Token { kind: '+', .. }) | Some(t @ Token { kind: '-', .. }) => {
                buf.push(t.kind);
                self.toks.next();
            }
            _ => {}
        }

        match self.toks.peek() {
            Some(t) if t.kind.is_ascii_digit() => {
                while let Some(t) = self.toks.peek() {
                    if !t.kind.is_ascii_digit() {
                        break;
                    }
                    buf.push(t.kind);
                    self.toks.next();
                }
                self.whitespace()?;
                if !self.scan_ident_char('n', false)? {
                    return Ok(buf);
                }
            }
            Some(..) => self.expect_ident_char('n', false)?,
            None => return Err(("expected more input.", self.span).into()),
        }

        buf.push('n');

        self.whitespace()?;

        if let Some(t @ Token { kind: '+', .. }) | Some(t @ Token { kind: '-', .. }) =
            self.toks.peek()
        {
            buf.push(t.kind);
            self.toks.next();
            self.whitespace()?;
            match self.toks.peek() {
                Some(t) if !t.kind.is_ascii_digit() => {
                    return Err(("Expected a number.", self.span).into());
                }
                None => return Err(("Expected a number.", self.span).into()),
                Some(..) => {}
            }

            while let Some(t) = self.toks.peek() {
                if !t.kind.is_ascii_digit() {
                    break;
                }
                buf.push(t.kind);
                self.toks.next();
            }
        }
        Ok(buf)
    }
}
