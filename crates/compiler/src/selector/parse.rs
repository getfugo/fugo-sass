use codemap::Span;

use crate::{
    Token,
    common::unvendor,
    error::SassResult,
    lexer::Lexer,
    parse::{BaseParser, IdentifierParser, TokenParser},
};

use super::{
    Attribute, Combinator, ComplexSelector, ComplexSelectorComponent, CompoundSelector, Namespace,
    Pseudo, QualifiedName, SelectorList, SimpleSelector,
};

#[derive(PartialEq)]
enum DevouredWhitespace {
    /// Some whitespace was found
    Whitespace,
    /// A newline and potentially other whitespace was found
    Newline,
    /// No whitespace was found
    None,
}

/// Pseudo-class selectors that take unadorned selectors as arguments.
const SELECTOR_PSEUDO_CLASSES: [&str; 9] = [
    "not",
    "matches",
    "where",
    "is",
    "current",
    "any",
    "has",
    "host",
    "host-context",
];

/// Pseudo-element selectors that take unadorned selectors as arguments.
const SELECTOR_PSEUDO_ELEMENTS: [&str; 1] = ["slotted"];

pub(crate) struct SelectorParser {
    /// Whether this parser allows the parent selector `&`.
    allows_parent: bool,

    /// Whether this parser allows placeholder selectors beginning with `%`.
    allows_placeholder: bool,

    /// Whether to parse the selector as plain CSS: `&` may be anywhere in a compound selector
    /// (CSS nesting), without a suffix, and placeholders are not allowed.
    plain_css: bool,

    pub toks: Lexer,

    span: Span,
}

mod simple;

impl BaseParser for SelectorParser {
    fn toks(&self) -> &Lexer {
        &self.toks
    }

    fn toks_mut(&mut self) -> &mut Lexer {
        &mut self.toks
    }
}

impl SelectorParser {
    pub fn new(toks: Lexer, allows_parent: bool, allows_placeholder: bool, span: Span) -> Self {
        Self {
            toks,
            allows_parent,
            allows_placeholder,
            plain_css: false,
            span,
        }
    }

    /// This parser parsing the selector as plain CSS.
    pub fn plain_css(mut self) -> Self {
        self.plain_css = true;
        self
    }

    pub fn parse(mut self) -> SassResult<SelectorList> {
        let tmp = self.parse_selector_list()?;
        if self.toks.peek().is_some() {
            return Err(("expected selector.", self.span).into());
        }
        Ok(tmp)
    }

    fn parse_selector_list(&mut self) -> SassResult<SelectorList> {
        let mut components = vec![self.parse_complex_selector(false)?];

        self.whitespace()?;

        let mut line_break = false;

        while self.scan_char(',') {
            line_break = self.eat_whitespace() == DevouredWhitespace::Newline || line_break;
            match self.toks.peek() {
                Some(Token { kind: ',', .. }) => continue,
                Some(..) => {}
                None => break,
            }
            components.push(self.parse_complex_selector(line_break)?);

            line_break = false;
        }

        Ok(SelectorList {
            components,
            span: self.span,
        })
    }

    fn eat_whitespace(&mut self) -> DevouredWhitespace {
        let text = self.raw_text(Self::whitespace);

        if text.contains('\n') {
            DevouredWhitespace::Newline
        } else if !text.is_empty() {
            DevouredWhitespace::Whitespace
        } else {
            DevouredWhitespace::None
        }
    }

    /// Consumes a complex selector.
    ///
    /// If `line_break` is `true`, that indicates that there was a line break
    /// before this selector.
    fn parse_complex_selector(&mut self, line_break: bool) -> SassResult<ComplexSelector> {
        let mut components = Vec::new();

        loop {
            self.whitespace()?;

            // todo: can we do while let Some(..) = self.toks.peek() ?
            match self.toks.peek() {
                Some(Token { kind: '+', .. }) => {
                    self.toks.next();
                    components.push(ComplexSelectorComponent::Combinator(
                        Combinator::NextSibling,
                    ));
                }
                Some(Token { kind: '>', .. }) => {
                    self.toks.next();
                    components.push(ComplexSelectorComponent::Combinator(Combinator::Child));
                }
                Some(Token { kind: '~', .. }) => {
                    self.toks.next();
                    components.push(ComplexSelectorComponent::Combinator(
                        Combinator::FollowingSibling,
                    ));
                }
                Some(Token { kind: '[', .. })
                | Some(Token { kind: '.', .. })
                | Some(Token { kind: '#', .. })
                | Some(Token { kind: '%', .. })
                | Some(Token { kind: ':', .. })
                // todo: ampersand?
                | Some(Token { kind: '&', .. })
                | Some(Token { kind: '*', .. })
                | Some(Token { kind: '|', .. }) => {
                    components.push(ComplexSelectorComponent::Compound(
                        self.parse_compound_selector()?,
                    ));
                    if let Some(Token { kind: '&', .. }) = self.toks.peek() {
                        return Err(("\"&\" may only used at the beginning of a compound selector.", self.span).into());
                    }
                }
                Some(..) => {
                    if !self.looking_at_identifier() {
                        break;
                    }
                    components.push(ComplexSelectorComponent::Compound(
                        self.parse_compound_selector()?,
                    ));
                    if let Some(Token { kind: '&', .. }) = self.toks.peek() {
                        return Err(("\"&\" may only used at the beginning of a compound selector.", self.span).into());
                    }
                }
                None => break,
            }
        }

        if components.is_empty()
            || (self.plain_css
                && matches!(
                    components.last(),
                    Some(ComplexSelectorComponent::Combinator(..))
                ))
        {
            return Err(("expected selector.", self.span).into());
        }

        Ok(ComplexSelector::new(components, line_break))
    }

    fn parse_compound_selector(&mut self) -> SassResult<CompoundSelector> {
        let mut components = vec![self.parse_simple_selector(None)?];

        while let Some(Token { kind, .. }) = self.toks.peek() {
            if !(is_simple_selector_start(kind) || (self.plain_css && kind == '&')) {
                break;
            }

            components.push(self.parse_simple_selector(Some(self.plain_css))?);
        }

        Ok(CompoundSelector { components })
    }

    /// Consumes a simple selector.
    ///
    /// If `allows_parent` is `Some`, this will override `self.allows_parent`. If `allows_parent`
    /// is `None`, it will fallback to `self.allows_parent`.
    fn parse_simple_selector(&mut self, allows_parent: Option<bool>) -> SassResult<SimpleSelector> {
        match self.toks.peek() {
            Some(Token { kind: '[', .. }) => self.parse_attribute_selector(),
            Some(Token { kind: '.', .. }) => self.parse_class_selector(),
            Some(Token { kind: '#', .. }) => self.parse_id_selector(),
            Some(Token { kind: '%', .. }) => {
                if self.plain_css {
                    return Err((
                        "Placeholder selectors aren't allowed in plain CSS.",
                        self.span,
                    )
                        .into());
                }
                if !self.allows_placeholder {
                    return Err(("Placeholder selectors aren't allowed here.", self.span).into());
                }
                self.parse_placeholder_selector()
            }
            Some(Token { kind: ':', .. }) => self.parse_pseudo_selector(),
            Some(Token { kind: '&', .. }) => {
                let allows_parent = allows_parent.unwrap_or(self.allows_parent);
                if !allows_parent {
                    return Err(("Parent selectors aren't allowed here.", self.span).into());
                }

                self.parse_parent_selector()
            }
            _ => self.parse_type_or_universal_selector(),
        }
    }
}

/// Returns whether `c` can start a simple selector other than a type
/// selector.
fn is_simple_selector_start(c: char) -> bool {
    matches!(c, '*' | '[' | '.' | '#' | '%' | ':')
}

/// Returns whether `name` is the name of a pseudo-element that can be written
/// with pseudo-class syntax (`:before`, `:after`, `:first-line`, or
/// `:first-letter`)
fn is_fake_pseudo_element(name: &str) -> bool {
    match name.as_bytes().first() {
        Some(b'a') | Some(b'A') => name.eq_ignore_ascii_case("after"),
        Some(b'b') | Some(b'B') => name.eq_ignore_ascii_case("before"),
        Some(b'f') | Some(b'F') => matches!(
            name.to_ascii_lowercase().as_str(),
            "first-line" | "first-letter"
        ),
        _ => false,
    }
}
