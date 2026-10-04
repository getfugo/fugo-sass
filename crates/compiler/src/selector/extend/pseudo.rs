//! Extending the selectors in pseudo-classes, and trimming redundant selectors.

use super::*;

impl ExtensionStore {
    /// Extends `pseudo` using `extensions`, and returns a list of resulting
    /// pseudo selectors.
    pub(super) fn extend_pseudo(
        &mut self,
        pseudo: Pseudo,
        extensions: Option<&HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>>,
        media_query_context: &Option<Vec<CssMediaQuery>>,
    ) -> Option<Vec<Pseudo>> {
        let extended = self.extend_list(
            pseudo
                .selector
                .as_deref()
                .cloned()
                .unwrap_or_else(|| SelectorList::new(self.span)),
            extensions,
            media_query_context,
        );
        /*todo: identical(extended, pseudo.selector)*/
        if Some(&extended) == pseudo.selector.as_deref() {
            return None;
        }

        // For `:not()`, we usually want to get rid of any complex selectors because
        // that will cause the selector to fail to parse on all browsers at time of
        // writing. We can keep them if either the original selector had a complex
        // selector, or the result of extending has only complex selectors, because
        // either way we aren't breaking anything that isn't already broken.
        let mut complexes = if pseudo.normalized_name() == "not"
            && !pseudo
                .selector
                .clone()
                .unwrap()
                .components
                .iter()
                .any(|complex| complex.components.len() > 1)
            && extended
                .components
                .iter()
                .any(|complex| complex.components.len() == 1)
        {
            extended
                .components
                .into_iter()
                .filter(|complex| complex.components.len() <= 1)
                .collect()
        } else {
            extended.components
        };

        complexes = complexes
            .into_iter()
            .flat_map(|complex| {
                if complex.components.len() != 1 {
                    return vec![complex];
                }
                let compound = match complex.components.first() {
                    Some(ComplexSelectorComponent::Compound(c)) => c,
                    Some(..) | None => return vec![complex],
                };
                if compound.components.len() != 1 {
                    return vec![complex];
                }
                if !compound.components.first().unwrap().is_pseudo() {
                    return vec![complex];
                }
                let inner_pseudo = match compound.components.first() {
                    Some(SimpleSelector::Pseudo(pseudo)) => pseudo,
                    Some(..) | None => return vec![complex],
                };
                if inner_pseudo.selector.is_none() {
                    return vec![complex];
                }

                match pseudo.normalized_name() {
                    "not" => {
                        // In theory, if there's a `:not` nested within another `:not`, the
                        // inner `:not`'s contents should be unified with the return value.
                        // For example, if `:not(.foo)` extends `.bar`, `:not(.bar)` should
                        // become `.foo:not(.bar)`. However, this is a narrow edge case and
                        // supporting it properly would make this code and the code calling it
                        // a lot more complicated, so it's not supported for now.
                        let inner_pseudo_normalized = inner_pseudo.normalized_name();
                        if ["matches", "is", "where"].contains(&inner_pseudo_normalized) {
                            inner_pseudo.selector.clone().unwrap().components
                        } else {
                            Vec::new()
                        }
                    }
                    "matches" | "where" | "is" | "any" | "current" | "nth-child"
                    | "nth-last-child" => {
                        // As above, we could theoretically support :not within :matches, but
                        // doing so would require this method and its callers to handle much
                        // more complex cases that likely aren't worth the pain.
                        if inner_pseudo.name != pseudo.name
                            || inner_pseudo.argument != pseudo.argument
                        {
                            Vec::new()
                        } else {
                            inner_pseudo.selector.clone().unwrap().components
                        }
                    }
                    "has" | "host" | "host-context" | "slotted" => {
                        // We can't expand nested selectors here, because each layer adds an
                        // additional layer of semantics. For example, `:has(:has(img))`
                        // doesn't match `<div><img></div>` but `:has(img)` does.
                        vec![complex]
                    }
                    _ => Vec::new(),
                }
            })
            .collect();

        // Older browsers support `:not`, but only with a single complex selector.
        // In order to support those browsers, we break up the contents of a `:not`
        // unless it originally contained a selector list.
        if pseudo.normalized_name() == "not"
            && pseudo.selector.clone().unwrap().components.len() == 1
        {
            let result = complexes
                .into_iter()
                .map(|complex| {
                    pseudo.clone().with_selector(Some(Box::new(SelectorList {
                        components: vec![complex],
                        span: self.span,
                    })))
                })
                .collect::<Vec<Pseudo>>();
            if result.is_empty() {
                None
            } else {
                Some(result)
            }
        } else {
            Some(vec![pseudo.with_selector(Some(Box::new(SelectorList {
                components: complexes,
                span: self.span,
            })))])
        }
    }

    /// Extends `simple` without extending the contents of any selector pseudos
    /// it contains.
    pub(super) fn without_pseudo(
        &self,
        simple: SimpleSelector,
        extensions: Option<&HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>>,
        targets_used: &mut HashSet<SimpleSelector>,
        mode: ExtendMode,
    ) -> Option<Vec<Extension>> {
        let extenders = extensions.unwrap_or(&self.extensions).get(&simple)?;

        targets_used.insert(simple.clone());

        if mode == ExtendMode::Replace {
            return Some(extenders.values().cloned().collect());
        }

        let mut tmp = vec![self.extension_for_simple(simple)];
        tmp.reserve(extenders.len());
        tmp.extend(extenders.values().cloned());

        Some(tmp)
    }

    /// Removes elements from `selectors` if they're subselectors of other
    /// elements.
    ///
    /// The `is_original` callback indicates which selectors are original to the
    /// document, and thus should never be trimmed.
    pub(super) fn trim(
        &self,
        selectors: Vec<ComplexSelector>,
        is_original: &dyn Fn(&ComplexSelector) -> bool,
    ) -> Vec<ComplexSelector> {
        // Avoid truly horrific quadratic behavior.
        //
        // TODO(nweiz): I think there may be a way to get perfect trimming without
        // going quadratic by building some sort of trie-like data structure that
        // can be used to look up superselectors.
        if selectors.len() > 100 {
            return selectors;
        }

        // This is n² on the sequences, but only comparing between separate
        // sequences should limit the quadratic behavior. We iterate from last to
        // first and reverse the result so that, if two selectors are identical, we
        // keep the first one.
        let mut result: VecDeque<ComplexSelector> = VecDeque::new();
        let mut num_originals = 0;

        // :outer
        for i in (0..=(selectors.len().saturating_sub(1))).rev() {
            let mut should_continue_to_outer = false;
            let complex1 = selectors.get(i).unwrap();
            if is_original(complex1) {
                // Make sure we don't include duplicate originals, which could happen if
                // a style rule extends a component of its own selector.
                for j in 0..num_originals {
                    if result.get(j) == Some(complex1) {
                        rotate_slice(&mut result, 0, j + 1);
                        should_continue_to_outer = true;
                        break;
                    }
                }
                if should_continue_to_outer {
                    continue;
                }
                num_originals += 1;
                result.push_front(complex1.clone());
                continue;
            }

            // The maximum specificity of the sources that caused `complex1` to be
            // generated. In order for `complex1` to be removed, there must be another
            // selector that's a superselector of it *and* that has specificity
            // greater or equal to this.
            let mut max_specificity = 0;
            for component in &complex1.components {
                if let ComplexSelectorComponent::Compound(compound) = component {
                    max_specificity = max_specificity.max(self.source_specificity_for(compound));
                }
            }

            // Look in `result` rather than `selectors` for selectors after `i`. This
            // ensures that we aren't comparing against a selector that's already been
            // trimmed, and thus that if there are two identical selectors only one is
            // trimmed.
            let should_continue = result.iter().any(|complex2| {
                complex2.min_specificity() >= max_specificity
                    && complex2.is_super_selector(complex1)
            });
            if should_continue {
                continue;
            }

            let should_continue = selectors.iter().take(i).any(|complex2| {
                complex2.min_specificity() >= max_specificity
                    && complex2.is_super_selector(complex1)
            });
            if should_continue {
                continue;
            }

            result.push_front(complex1.clone());
        }

        Vec::from(result)
    }
}
