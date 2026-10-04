//! Pseudo-class and pseudo-element selectors.

use super::*;

#[derive(Clone, Debug)]
pub(crate) struct Pseudo {
    /// The name of this selector.
    pub name: String,

    /// Whether this is a pseudo-class selector.
    ///
    /// If this is false, this is a pseudo-element selector
    pub is_class: bool,

    /// Whether this is syntactically a pseudo-class selector.
    ///
    /// This is the same as `is_class` unless this selector is a pseudo-element
    /// that was written syntactically as a pseudo-class (`:before`, `:after`,
    /// `:first-line`, or `:first-letter`).
    ///
    /// If this is false, it is syntactically a psuedo-element
    pub is_syntactic_class: bool,

    /// The non-selector argument passed to this selector.
    ///
    /// This is `None` if there's no argument. If `argument` and `selector` are
    /// both non-`None`, the selector follows the argument.
    pub argument: Option<Box<str>>,

    /// The selector argument passed to this selector.
    ///
    /// This is `None` if there's no selector. If `argument` and `selector` are
    /// both non-`None`, the selector follows the argument.
    pub selector: Option<Box<SelectorList>>,

    pub span: Span,
}

impl PartialEq for Pseudo {
    fn eq(&self, other: &Pseudo) -> bool {
        self.name == other.name
            && self.is_class == other.is_class
            && self.argument == other.argument
            && self.selector == other.selector
    }
}

impl Eq for Pseudo {}

impl Hash for Pseudo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.is_class.hash(state);
        self.argument.hash(state);
        self.selector.hash(state);
    }
}

impl fmt::Display for Pseudo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(sel) = &self.selector
            && self.name == "not"
            && sel.is_invisible()
        {
            return Ok(());
        }

        f.write_char(':')?;

        if !self.is_syntactic_class {
            f.write_char(':')?;
        }

        f.write_str(&self.name)?;

        if self.argument.is_none() && self.selector.is_none() {
            return Ok(());
        }

        f.write_char('(')?;
        if let Some(arg) = &self.argument {
            f.write_str(arg)?;
            if self.selector.is_some() {
                f.write_char(' ')?;
            }
        }

        if let Some(sel) = &self.selector {
            write!(f, "{}", sel)?;
        }

        f.write_char(')')
    }
}

impl Pseudo {
    /// Returns whether `pseudo1` is a superselector of `compound2`.
    ///
    /// That is, whether `pseudo1` matches every element that `compound2` matches, as well
    /// as possibly additional elements.
    ///
    /// This assumes that `pseudo1`'s `selector` argument is not `None`.
    ///
    /// If `parents` is passed, it represents the parents of `compound`. This is
    /// relevant for pseudo selectors with selector arguments, where we may need to
    /// know if the parent selectors in the selector argument match `parents`.
    pub fn is_super_selector(
        &self,
        compound: &CompoundSelector,
        parents: Option<Vec<ComplexSelectorComponent>>,
    ) -> bool {
        debug_assert!(self.selector.is_some());
        match self.normalized_name() {
            "matches" | "is" | "any" | "where" => {
                selector_pseudos_named(compound.clone(), &self.name, true).any(move |pseudo2| {
                    self.selector
                        .as_ref()
                        .unwrap()
                        .is_superselector(&pseudo2.selector.unwrap())
                }) || self
                    .selector
                    .as_ref()
                    .unwrap()
                    .components
                    .iter()
                    .any(move |complex1| {
                        let mut components = parents.clone().unwrap_or_default();
                        components.push(ComplexSelectorComponent::Compound(compound.clone()));
                        complex1.is_super_selector(&ComplexSelector::new(components, false))
                    })
            }
            "has" | "host" | "host-context" => {
                selector_pseudos_named(compound.clone(), &self.name, true).any(|pseudo2| {
                    self.selector
                        .as_ref()
                        .unwrap()
                        .is_superselector(&pseudo2.selector.unwrap())
                })
            }
            "slotted" => {
                selector_pseudos_named(compound.clone(), &self.name, false).any(|pseudo2| {
                    self.selector
                        .as_ref()
                        .unwrap()
                        .is_superselector(pseudo2.selector.as_ref().unwrap())
                })
            }
            "not" => self
                .selector
                .as_ref()
                .unwrap()
                .components
                .iter()
                .all(|complex| {
                    compound.components.iter().any(|simple2| {
                        if let SimpleSelector::Type(..) = simple2 {
                            let compound1 = complex.components.last();
                            if let Some(ComplexSelectorComponent::Compound(c)) = compound1 {
                                c.components
                                    .iter()
                                    .any(|simple1| simple1.is_type() && simple1 != simple2)
                            } else {
                                false
                            }
                        } else if let SimpleSelector::Id(..) = simple2 {
                            let compound1 = complex.components.last();
                            if let Some(ComplexSelectorComponent::Compound(c)) = compound1 {
                                c.components
                                    .iter()
                                    .any(|simple1| simple1.is_id() && simple1 != simple2)
                            } else {
                                false
                            }
                        } else if let SimpleSelector::Pseudo(Pseudo {
                            selector: Some(sel),
                            name,
                            ..
                        }) = simple2
                        {
                            if name != &self.name {
                                return false;
                            }
                            sel.is_superselector(&SelectorList {
                                components: vec![complex.clone()],
                                span: self.span,
                            })
                        } else {
                            false
                        }
                    })
                }),
            "current" => selector_pseudos_named(compound.clone(), &self.name, self.is_class)
                .any(|pseudo2| self.selector == pseudo2.selector),
            "nth-child" | "nth-last-child" => compound.components.iter().any(|pseudo2| {
                if let SimpleSelector::Pseudo(
                    pseudo @ Pseudo {
                        selector: Some(..), ..
                    },
                ) = pseudo2
                {
                    pseudo.name == self.name
                        && pseudo.argument == self.argument
                        && self
                            .selector
                            .as_ref()
                            .unwrap()
                            .is_superselector(pseudo.selector.as_ref().unwrap())
                } else {
                    false
                }
            }),
            _ => unreachable!(),
        }
    }

    #[allow(clippy::missing_const_for_fn)]
    pub fn with_selector(self, selector: Option<Box<SelectorList>>) -> Self {
        Self { selector, ..self }
    }

    pub fn max_specificity(&self) -> i32 {
        self.specificity().max
    }

    pub fn min_specificity(&self) -> i32 {
        self.specificity().min
    }

    pub fn specificity(&self) -> Specificity {
        if !self.is_class {
            return Specificity { min: 1, max: 1 };
        }

        let selector = match &self.selector {
            Some(sel) => sel,
            None => {
                return Specificity {
                    min: BASE_SPECIFICITY,
                    max: BASE_SPECIFICITY,
                };
            }
        };

        if self.name == "not" {
            let mut min = 0;
            let mut max = 0;
            for complex in &selector.components {
                min = min.max(complex.min_specificity());
                max = max.max(complex.max_specificity());
            }
            Specificity { min, max }
        } else {
            // This is higher than any selector's specificity can actually be.
            let mut min = BASE_SPECIFICITY.pow(3_u32);
            let mut max = 0;
            for complex in &selector.components {
                min = min.min(complex.min_specificity());
                max = max.max(complex.max_specificity());
            }
            Specificity { min, max }
        }
    }

    /// Like `name`, but without any vendor prefixes.
    pub fn normalized_name(&self) -> &str {
        unvendor(&self.name)
    }
}

/// Returns all pseudo selectors in `compound` that have a selector argument,
/// and that have the given `name`.
pub(super) fn selector_pseudos_named(
    compound: CompoundSelector,
    name: &str,
    is_class: bool,
) -> impl Iterator<Item = Pseudo> + '_ {
    compound
        .components
        .into_iter()
        .filter_map(|c| {
            if let SimpleSelector::Pseudo(p) = c {
                Some(p)
            } else {
                None
            }
        })
        .filter(move |p| p.is_class == is_class && p.selector.is_some() && p.name == name)
}
