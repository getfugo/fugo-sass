use std::{
    fmt::{self, Write},
    hash::{Hash, Hasher},
};

use codemap::Span;

use crate::{common::unvendor, error::SassResult};

use super::{
    Attribute, ComplexSelector, ComplexSelectorComponent, CompoundSelector, Namespace,
    QualifiedName, SelectorList, Specificity,
};

mod pseudo;
mod unify;

pub(crate) use pseudo::Pseudo;

const SUBSELECTOR_PSEUDOS: [&str; 6] = [
    "matches",
    "where",
    "is",
    "any",
    "nth-child",
    "nth-last-child",
];

const BASE_SPECIFICITY: i32 = 1000;

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub(crate) enum SimpleSelector {
    /// *
    Universal(Namespace),

    /// A pseudo-class or pseudo-element selector.
    ///
    /// The semantics of a specific pseudo selector depends on its name. Some
    /// selectors take arguments, including other selectors. Sass manually encodes
    /// logic for each pseudo selector that takes a selector as an argument, to
    /// ensure that extension and other selector operations work properly.
    Pseudo(Pseudo),

    /// A type selector.
    ///
    /// This selects elements whose name equals the given name.
    Type(QualifiedName),

    /// A placeholder selector.
    ///
    /// This doesn't match any elements. It's intended to be extended using
    /// `@extend`. It's not a plain CSS selector—it should be removed before
    /// emitting a CSS document.
    Placeholder(String),

    /// A selector that matches the parent in the Sass stylesheet.
    /// `&`
    ///
    /// This is not a plain CSS selector—it should be removed before emitting a CSS
    /// document.
    ///
    /// The parameter is the suffix that will be added to the parent selector after
    /// it's been resolved.
    ///
    /// This is assumed to be a valid identifier suffix. It may be `None`,
    /// indicating that the parent selector will not be modified.
    Parent(Option<String>),

    Id(String),

    /// A class selector.
    ///
    /// This selects elements whose `class` attribute contains an identifier with
    /// the given name.
    Class(String),

    Attribute(Box<Attribute>),
}

impl fmt::Display for SimpleSelector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Id(name) => write!(f, "#{}", name),
            Self::Class(name) => write!(f, ".{}", name),
            Self::Placeholder(name) => write!(f, "%{}", name),
            Self::Universal(namespace) => write!(f, "{}*", namespace),
            Self::Pseudo(pseudo) => write!(f, "{}", pseudo),
            Self::Type(name) => write!(f, "{}", name),
            Self::Attribute(attr) => write!(f, "{}", attr),
            Self::Parent(suffix) => write!(f, "&{}", suffix.as_deref().unwrap_or("")),
        }
    }
}

impl SimpleSelector {
    /// The minimum possible specificity that this selector can have.
    ///
    /// Pseudo selectors that contain selectors, like `:not()` and `:matches()`,
    /// can have a range of possible specificities.
    ///
    /// Specifity is represented in base 1000. The spec says this should be
    /// "sufficiently high"; it's extremely unlikely that any single selector
    /// sequence will contain 1000 simple selectors.
    pub fn min_specificity(&self) -> i32 {
        match self {
            Self::Universal(..) => 0,
            Self::Type(..) => 1,
            Self::Pseudo(pseudo) => pseudo.min_specificity(),
            Self::Id(..) => BASE_SPECIFICITY.pow(2_u32),
            _ => BASE_SPECIFICITY,
        }
    }

    /// The maximum possible specificity that this selector can have.
    ///
    /// Pseudo selectors that contain selectors, like `:not()` and `:matches()`,
    /// can have a range of possible specificities.
    pub fn max_specificity(&self) -> i32 {
        match self {
            Self::Universal(..) => 0,
            Self::Pseudo(pseudo) => pseudo.max_specificity(),
            _ => self.min_specificity(),
        }
    }

    pub fn is_invisible(&self) -> bool {
        match self {
            Self::Universal(..)
            | Self::Type(..)
            | Self::Id(..)
            | Self::Class(..)
            | Self::Attribute(..) => false,
            Self::Pseudo(Pseudo { name, selector, .. }) => {
                name != "not" && selector.as_ref().is_some_and(|sel| sel.is_invisible())
            }
            Self::Placeholder(..) => true,
            // A parent selector left at the root of the document is written as is.
            Self::Parent(..) => false,
        }
    }

    pub fn add_suffix(&mut self, suffix: &str, span: Span) -> SassResult<()> {
        match self {
            Self::Type(name) => name.ident.push_str(suffix),
            Self::Placeholder(name)
            | Self::Id(name)
            | Self::Class(name)
            | Self::Pseudo(Pseudo {
                name,
                argument: None,
                selector: None,
                ..
            }) => name.push_str(suffix),
            // todo: add test for this?
            _ => return Err((format!("Invalid parent selector \"{}\"", self), span).into()),
        };
        Ok(())
    }

    pub fn is_universal(&self) -> bool {
        matches!(self, Self::Universal(..))
    }

    pub fn is_pseudo(&self) -> bool {
        matches!(self, Self::Pseudo { .. })
    }

    pub fn is_parent(&self) -> bool {
        matches!(self, Self::Parent(..))
    }

    pub fn is_id(&self) -> bool {
        matches!(self, Self::Id(..))
    }

    pub fn is_type(&self) -> bool {
        matches!(self, Self::Type(..))
    }

    pub fn is_super_selector_of_compound(&self, compound: &CompoundSelector) -> bool {
        compound.components.iter().any(|their_simple| {
            if self == their_simple {
                return true;
            }
            if let SimpleSelector::Pseudo(Pseudo {
                selector: Some(sel),
                name,
                ..
            }) = their_simple
            {
                if SUBSELECTOR_PSEUDOS.contains(&unvendor(name)) {
                    return sel.components.iter().all(|complex| {
                        if complex.components.len() != 1 {
                            return false;
                        };
                        complex
                            .components
                            .first()
                            .unwrap()
                            .as_compound()
                            .components
                            .contains(self)
                    });
                }
                false
            } else {
                false
            }
        })
    }
}
