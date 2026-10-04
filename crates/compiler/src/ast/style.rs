use codemap::{Span, Spanned};

use crate::{interner::InternedString, value::Value};

/// A style: `color: red`
#[derive(Clone, Debug)]
pub(crate) struct Style {
    pub property: InternedString,
    pub value: Box<Spanned<Value>>,
    pub declared_as_custom_property: bool,
    /// Where the property's name is, whose column a custom property's value is reindented from.
    pub name_span: Span,
}
