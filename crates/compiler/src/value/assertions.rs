//! Asserting a value's type for a function's argument, and converting values to selectors.

use super::*;

impl Value {
    pub fn assert_number(self, span: Span) -> SassResult<SassNumber> {
        match self {
            Value::Dimension(n) => Ok(n),
            _ => Err((format!("{} is not a number.", self.inspect(span)?), span).into()),
        }
    }

    pub fn assert_number_with_name(self, name: &str, span: Span) -> SassResult<SassNumber> {
        match self {
            Value::Dimension(n) => Ok(n),
            _ => Err((
                format!(
                    "${name}: {} is not a number.",
                    self.inspect(span)?,
                    name = name,
                ),
                span,
            )
                .into()),
        }
    }

    pub fn assert_color_with_name(self, name: &str, span: Span) -> SassResult<Arc<Color>> {
        match self {
            Value::Color(c) => Ok(c),
            _ => Err((
                format!(
                    "${name}: {} is not a color.",
                    self.inspect(span)?,
                    name = name,
                ),
                span,
            )
                .into()),
        }
    }

    pub fn assert_map_with_name(self, name: &str, span: Span) -> SassResult<SassMap> {
        match self {
            Value::Map(m) => Ok(m),
            Value::List(v, ..) if v.is_empty() => Ok(SassMap::new()),
            Value::ArgList(v) if v.is_empty() => Ok(SassMap::new()),
            _ => Err((
                format!(
                    "${name}: {} is not a map.",
                    self.inspect(span)?,
                    name = name,
                ),
                span,
            )
                .into()),
        }
    }

    pub fn assert_string_with_name(
        self,
        name: &str,
        span: Span,
    ) -> SassResult<(String, QuoteKind)> {
        match self {
            Value::String(s, quotes) => Ok((s, quotes)),
            _ => Err((
                format!(
                    "${name}: {} is not a string.",
                    self.inspect(span)?,
                    name = name,
                ),
                span,
            )
                .into()),
        }
    }

    /// Parses `self` as a selector list, in the same manner as the
    /// `selector-parse()` function.
    ///
    /// Returns a `SassError` if `self` isn't a type that can be parsed as a
    /// selector, or if parsing fails. If `allow_parent` is `true`, this allows
    /// parent selectors. Otherwise, they're considered parse errors.
    ///
    /// `name` is the argument name, which prefixes the errors (`$name: …`) as dart-sass's
    /// `assertSelector(name: …)` does; `selector.nest()` and `selector.append()` pass none.
    pub fn to_selector(
        self,
        visitor: &mut Visitor,
        name: Option<&str>,
        allows_parent: bool,
        span: Span,
    ) -> SassResult<Selector> {
        let named = |e: Box<SassError>| match name {
            Some(name) => e.with_argument_name(name),
            None => e,
        };
        let string = match self.clone().selector_string()? {
            Some(v) => v,
            None => return Err(named((format!("{} is not a valid selector: it must be a string,\n a list of strings, or a list of lists of strings.", self.inspect(span)?), span).into())),
        };
        Ok(Selector(
            visitor
                .parse_selector_from_string(&string, allows_parent, true, span)
                .map_err(named)?,
        ))
    }

    pub(super) fn selector_string(self) -> SassResult<Option<String>> {
        Ok(Some(match self {
            Value::String(text, ..) => text,
            Value::List(list, sep, ..) if !list.is_empty() => {
                let mut result = Vec::new();
                match sep {
                    ListSeparator::Comma => {
                        for complex in list {
                            if let Value::String(text, ..) = complex {
                                result.push(text);
                            } else if let Value::List(
                                _,
                                ListSeparator::Space | ListSeparator::Undecided,
                                ..,
                            ) = complex
                            {
                                result.push(match complex.selector_string()? {
                                    Some(v) => v,
                                    None => return Ok(None),
                                });
                            } else {
                                return Ok(None);
                            }
                        }
                    }
                    ListSeparator::Slash => return Ok(None),
                    ListSeparator::Space | ListSeparator::Undecided => {
                        for compound in list {
                            if let Value::String(text, ..) = compound {
                                result.push(text);
                            } else {
                                return Ok(None);
                            }
                        }
                    }
                }

                result.join(sep.as_str())
            }
            _ => return Ok(None),
        }))
    }
}
