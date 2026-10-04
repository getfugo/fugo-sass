//! Extending selector lists, complex and compound selectors with extensions.

use super::*;

impl ExtensionStore {
    /// Extends `list` using `extensions`.
    pub(super) fn extend_list(
        &mut self,
        list: SelectorList,
        extensions: Option<&HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>>,
        media_query_context: &Option<Vec<CssMediaQuery>>,
    ) -> SelectorList {
        // This could be written more simply using Vec<Vec<T>>, but we want to avoid
        // any allocations in the common case where no extends apply.
        let mut extended: Option<Vec<ComplexSelector>> = None;
        for (i, complex) in list.components.iter().enumerate() {
            if let Some(result) =
                self.extend_complex(complex.clone(), extensions, media_query_context)
            {
                if extended.is_none() {
                    extended = Some(if i == 0 {
                        Vec::new()
                    } else {
                        list.components[0..i].to_vec()
                    });
                }
                match extended.as_mut() {
                    Some(v) => v.extend(result),
                    None => unreachable!(),
                }
            } else if let Some(extended) = extended.as_mut() {
                extended.push(complex.clone());
            }
        }

        let extended = match extended {
            Some(v) => v,
            None => return list,
        };

        SelectorList {
            components: self.trim(extended, &|complex| self.originals.contains(complex)),
            span: self.span,
        }
    }

    /// Extends `complex` using `extensions`, and returns the contents of a
    /// `SelectorList`.
    pub(super) fn extend_complex(
        &mut self,
        complex: ComplexSelector,
        extensions: Option<&HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>>,
        media_query_context: &Option<Vec<CssMediaQuery>>,
    ) -> Option<Vec<ComplexSelector>> {
        // The complex selectors that each compound selector in `complex.components`
        // can expand to.
        //
        // For example, given
        //
        //     .a .b {...}
        //     .x .y {@extend .b}
        //
        // this will contain
        //
        //     [
        //       [.a],
        //       [.b, .x .y]
        //     ]
        //
        // This could be written more simply using `Vec::into_iter::map`, but we want to avoid
        // any allocations in the common case where no extends apply.
        let mut extended_not_expanded: Option<Vec<Vec<ComplexSelector>>> = None;

        let complex_has_line_break = complex.line_break;

        let is_original = self.originals.contains(&complex);

        for (i, component) in complex.components.iter().enumerate() {
            if let ComplexSelectorComponent::Compound(component) = component {
                if let Some(extended) =
                    self.extend_compound(component, extensions, media_query_context, is_original)
                {
                    if extended_not_expanded.is_none() {
                        extended_not_expanded = Some(
                            complex
                                .components
                                .clone()
                                .into_iter()
                                .take(i)
                                .map(|component| {
                                    vec![ComplexSelector::new(vec![component], complex.line_break)]
                                })
                                .collect(),
                        );
                    }
                    match extended_not_expanded.as_mut() {
                        Some(v) => v.push(extended),
                        None => unreachable!(),
                    }
                } else {
                    match extended_not_expanded.as_mut() {
                        Some(v) => v.push(vec![ComplexSelector::new(
                            vec![ComplexSelectorComponent::Compound(component.clone())],
                            false,
                        )]),
                        None => {}
                    }
                }
            } else if component.is_combinator() {
                match extended_not_expanded.as_mut() {
                    Some(v) => v.push(vec![ComplexSelector::new(vec![component.clone()], false)]),
                    None => {}
                }
            }
        }

        let extended_not_expanded = extended_not_expanded?;

        let mut first = true;

        Some(
            paths(extended_not_expanded)
                .into_iter()
                .flat_map(move |path| {
                    weave(
                        path.clone()
                            .into_iter()
                            .map(move |complex| complex.components)
                            .collect(),
                    )
                    .into_iter()
                    .map(|components| {
                        let output_complex = ComplexSelector::new(
                            components,
                            complex_has_line_break
                                || path.iter().any(|input_complex| input_complex.line_break),
                        );

                        // Make sure that copies of `complex` retain their status as "original"
                        // selectors. This includes selectors that are modified because a :not()
                        // was extended into.
                        if first && self.originals.contains(&complex) {
                            self.originals.insert(&output_complex);
                        }
                        first = false;

                        output_complex
                    })
                    .collect::<Vec<ComplexSelector>>()
                })
                .collect(),
        )
    }

    /// Extends `compound` using `extensions`, and returns the contents of a
    /// `SelectorList`.
    ///
    /// The `in_original` parameter indicates whether this is in an original
    /// complex selector, meaning that `compound` should not be trimmed out.
    pub(super) fn extend_compound(
        &mut self,
        compound: &CompoundSelector,
        extensions: Option<&HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>>,
        media_query_context: &Option<Vec<CssMediaQuery>>,
        in_original: bool,
    ) -> Option<Vec<ComplexSelector>> {
        // If there's more than one target and they all need to match, we track
        // which targets are actually extended.
        let mut targets_used: HashSet<SimpleSelector> = HashSet::new();

        let mut options: Option<Vec<Vec<Extension>>> = None;

        for i in 0..compound.components.len() {
            let simple = compound.components.get(i).cloned().unwrap();

            match self.extend_simple(
                simple.clone(),
                extensions,
                media_query_context,
                &mut targets_used,
            ) {
                Some(extended) => {
                    if options.is_none() {
                        let mut new_options = Vec::new();
                        if i != 0 {
                            new_options.push(vec![
                                self.extension_for_compound(compound.components[..i].to_vec()),
                            ]);
                        }
                        options.replace(new_options);
                    }

                    match options.as_mut() {
                        Some(v) => v.extend(extended),
                        None => unreachable!(),
                    }
                }
                None => match options.as_mut() {
                    Some(v) => v.push(vec![self.extension_for_simple(simple)]),
                    None => {}
                },
            }
        }

        let options = options?;

        // If `self.mode` isn't `ExtendMode::Normal` and we didn't use all the targets in
        // `extensions`, extension fails for `compound`.
        // todo: test for `extensions.len() > 2`. may cause issues
        if !targets_used.is_empty()
            && targets_used.len() != extensions.map_or(self.extensions.len(), HashMap::len)
            && self.mode != ExtendMode::Normal
        {
            return None;
        }

        // Optimize for the simple case of a single simple selector that doesn't
        // need any unification.
        if options.len() == 1 {
            return Some(
                options
                    .first()?
                    .clone()
                    .into_iter()
                    .map(|state| {
                        state.assert_compatible_media_context(media_query_context);
                        state.extender
                    })
                    .collect(),
            );
        }

        // Find all paths through `options`. In this case, each path represents a
        // different unification of the base selector. For example, if we have:
        //
        //     .a.b {...}
        //     .w .x {@extend .a}
        //     .y .z {@extend .b}
        //
        // then `options` is `[[.a, .w .x], [.b, .y .z]]` and `paths(options)` is
        //
        //     [
        //       [.a, .b],
        //       [.a, .y .z],
        //       [.w .x, .b],
        //       [.w .x, .y .z]
        //     ]
        //
        // We then unify each path to get a list of complex selectors:
        //
        //     [
        //       [.a.b],
        //       [.y .a.z],
        //       [.w .x.b],
        //       [.w .y .x.z, .y .w .x.z]
        //     ]
        let mut first = self.mode != ExtendMode::Replace;

        let unified_paths = paths(options).into_iter().map(|path| {
            let complexes: Vec<Vec<ComplexSelectorComponent>> = if first {
                // The first path is always the original selector. We can't just
                // return `compound` directly because pseudo selectors may be
                // modified, but we don't have to do any unification.
                first = false;

                vec![vec![ComplexSelectorComponent::Compound(CompoundSelector {
                    components: path
                        .clone()
                        .into_iter()
                        .flat_map(|state| {
                            debug_assert!(state.extender.components.len() == 1);
                            match state.extender.components.last().cloned() {
                                Some(ComplexSelectorComponent::Compound(c)) => c.components,
                                Some(..) | None => unreachable!(),
                            }
                        })
                        .collect(),
                })]]
            } else {
                let mut to_unify: VecDeque<Vec<ComplexSelectorComponent>> = VecDeque::new();
                let mut originals: Vec<SimpleSelector> = Vec::new();

                for state in path.clone() {
                    if state.is_original {
                        originals.extend(match state.extender.components.last().cloned() {
                            Some(ComplexSelectorComponent::Compound(c)) => c.components,
                            Some(..) | None => unreachable!(),
                        });
                    } else {
                        to_unify.push_back(state.extender.components.clone());
                    }
                }
                if !originals.is_empty() {
                    to_unify.push_front(vec![ComplexSelectorComponent::Compound(
                        CompoundSelector {
                            components: originals,
                        },
                    )]);
                }

                unify_complex(Vec::from(to_unify))?
            };

            let mut line_break = false;

            for state in path {
                state.assert_compatible_media_context(media_query_context);
                line_break = line_break || state.extender.line_break;
            }

            Some(
                complexes
                    .into_iter()
                    .map(|components| ComplexSelector::new(components, line_break))
                    .collect::<Vec<ComplexSelector>>(),
            )
        });

        let unified_paths: Vec<ComplexSelector> = unified_paths.flatten().flatten().collect();

        Some(if in_original && self.mode != ExtendMode::Replace {
            let original = unified_paths.first().cloned();
            self.trim(unified_paths, &|complex| Some(complex) == original.as_ref())
        } else {
            self.trim(unified_paths, &|_| false)
        })
    }

    pub(super) fn extend_simple(
        &mut self,
        simple: SimpleSelector,
        extensions: Option<&HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>>,
        media_query_context: &Option<Vec<CssMediaQuery>>,
        targets_used: &mut HashSet<SimpleSelector>,
    ) -> Option<Vec<Vec<Extension>>> {
        if let SimpleSelector::Pseudo(Pseudo {
            selector: Some(..), ..
        }) = &simple
        {
            let simple = if let SimpleSelector::Pseudo(pseudo) = simple.clone() {
                pseudo
            } else {
                unreachable!()
            };
            if let Some(extended) = self.extend_pseudo(simple, extensions, media_query_context) {
                return Some(
                    extended
                        .into_iter()
                        .map(move |pseudo| {
                            self.without_pseudo(
                                SimpleSelector::Pseudo(pseudo.clone()),
                                extensions,
                                targets_used,
                                self.mode,
                            )
                            .unwrap_or_else(|| {
                                vec![self.extension_for_simple(SimpleSelector::Pseudo(pseudo))]
                            })
                        })
                        .collect(),
                );
            }
        }

        self.without_pseudo(simple, extensions, targets_used, self.mode)
            .map(|v| vec![v])
    }

    /// Returns a one-off `Extension` whose extender is composed solely of
    /// `simple`.
    pub(super) fn extension_for_simple(&self, simple: SimpleSelector) -> Extension {
        let specificity = Some(*self.source_specificity.get(&simple).unwrap_or(&0_i32));
        Extension::one_off(
            ComplexSelector::new(
                vec![ComplexSelectorComponent::Compound(CompoundSelector {
                    components: vec![simple],
                })],
                false,
            ),
            specificity,
            true,
            self.span,
        )
    }

    /// Returns a one-off `Extension` whose extender is composed solely of a
    /// compound selector containing `simples`.
    pub(super) fn extension_for_compound(&self, simples: Vec<SimpleSelector>) -> Extension {
        let compound = CompoundSelector {
            components: simples,
        };
        let specificity = Some(self.source_specificity_for(&compound));
        Extension::one_off(
            ComplexSelector::new(vec![ComplexSelectorComponent::Compound(compound)], false),
            specificity,
            true,
            self.span,
        )
    }

    /// Returns the maximum specificity for sources that went into producing
    /// `compound`.
    pub(super) fn source_specificity_for(&self, compound: &CompoundSelector) -> i32 {
        let mut specificity = 0;
        for simple in &compound.components {
            specificity = specificity.max(*self.source_specificity.get(simple).unwrap_or(&0));
        }
        specificity
    }
}
