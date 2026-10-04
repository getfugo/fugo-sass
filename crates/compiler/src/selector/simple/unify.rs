//! Unifying simple selectors with compound selectors.

use super::*;

impl SimpleSelector {
    pub fn unify(self, compound: Vec<Self>) -> Option<Vec<Self>> {
        match self {
            Self::Type(..) => self.unify_type(compound),
            Self::Universal(..) => self.unify_universal(compound),
            Self::Pseudo { .. } => self.unify_pseudo(compound),
            Self::Id(..) => {
                if compound
                    .iter()
                    .any(|simple| simple.is_id() && simple != &self)
                {
                    return None;
                }

                self.unify_default(compound)
            }
            _ => self.unify_default(compound),
        }
    }

    /// Returns the components of a `CompoundSelector` that matches only elements
    /// matched by both this and `compound`.
    ///
    /// By default, this just returns a copy of `compound` with this selector
    /// added to the end, or returns the original array if this selector already
    /// exists in it.
    ///
    /// Returns `None` if unification is impossible—for example, if there are
    /// multiple ID selectors.
    pub(super) fn unify_default(self, mut compound: Vec<Self>) -> Option<Vec<Self>> {
        if compound.len() == 1 && compound[0].is_universal() {
            return compound.swap_remove(0).unify(vec![self]);
        }
        if compound.contains(&self) {
            return Some(compound);
        }
        let mut result: Vec<SimpleSelector> = Vec::new();
        let mut added_this = false;
        for simple in compound {
            if !added_this && simple.is_pseudo() {
                result.push(self.clone());
                added_this = true;
            }
            result.push(simple);
        }

        if !added_this {
            result.push(self);
        }

        Some(result)
    }

    pub(super) fn unify_universal(self, mut compound: Vec<Self>) -> Option<Vec<Self>> {
        if let Self::Universal(..) | Self::Type(..) = compound[0] {
            let mut unified = vec![self.unify_universal_and_element(&compound[0])?];
            unified.extend(compound.into_iter().skip(1));
            return Some(unified);
        }

        if self != Self::Universal(Namespace::Asterisk) && self != Self::Universal(Namespace::None)
        {
            let mut v = vec![self];
            v.append(&mut compound);
            return Some(v);
        }

        if !compound.is_empty() {
            return Some(compound);
        }

        Some(vec![self])
    }

    /// Returns a `SimpleSelector` that matches only elements that are matched by
    /// both `selector1` and `selector2`, which must both be either
    /// `SimpleSelector::Universal`s or `SimpleSelector::Type`s.
    ///
    /// If no such selector can be produced, returns `None`.
    pub(super) fn unify_universal_and_element(&self, other: &Self) -> Option<Self> {
        let namespace1;
        let name1;
        if let SimpleSelector::Type(name) = self.clone() {
            namespace1 = name.namespace;
            name1 = name.ident;
        } else if let SimpleSelector::Universal(namespace) = self.clone() {
            namespace1 = namespace;
            name1 = String::new();
        } else {
            unreachable!("{:?} must be a universal selector or a type selector", self);
        }

        let namespace2;
        let mut name2 = String::new();

        if let SimpleSelector::Universal(namespace) = other {
            namespace2 = namespace.clone();
        } else if let SimpleSelector::Type(name) = other {
            namespace2 = name.namespace.clone();
            name2 = name.ident.clone();
        } else {
            unreachable!(
                "{:?} must be a universal selector or a type selector",
                other
            );
        }

        let namespace = if namespace1 == namespace2 || namespace2 == Namespace::Asterisk {
            namespace1
        } else if namespace1 == Namespace::Asterisk {
            namespace2
        } else {
            return None;
        };

        let name = if name1 == name2 || name2.is_empty() {
            name1
        } else if name1.is_empty() || name1 == "*" {
            name2
        } else {
            return None;
        };

        Some(if name.is_empty() {
            SimpleSelector::Universal(namespace)
        } else {
            SimpleSelector::Type(QualifiedName {
                namespace,
                ident: name,
            })
        })
    }

    pub(super) fn unify_type(self, mut compound: Vec<Self>) -> Option<Vec<Self>> {
        if let Self::Universal(..) | Self::Type(..) = compound[0] {
            let mut unified = vec![self.unify_universal_and_element(&compound[0])?];
            unified.extend(compound.into_iter().skip(1));
            Some(unified)
        } else {
            let mut unified = vec![self];
            unified.append(&mut compound);
            Some(unified)
        }
    }

    pub(super) fn unify_pseudo(self, mut compound: Vec<Self>) -> Option<Vec<Self>> {
        if compound.len() == 1 && compound[0].is_universal() {
            return compound.remove(0).unify(vec![self]);
        }
        if compound.contains(&self) {
            return Some(compound);
        }

        let mut result = Vec::new();

        let mut added_self = false;

        for simple in compound {
            if let Self::Pseudo(Pseudo {
                is_class: false, ..
            }) = simple
            {
                // A given compound selector may only contain one pseudo element. If
                // `compound` has a different one than `self`, unification fails.
                if let Self::Pseudo(Pseudo {
                    is_class: false, ..
                }) = self
                {
                    return None;
                }

                // Otherwise, this is a pseudo selector and should come before pseduo
                // elements.
                result.push(self.clone());
                added_self = true;
            }
            result.push(simple);
        }

        if !added_self {
            result.push(self);
        }

        Some(result)
    }
}
