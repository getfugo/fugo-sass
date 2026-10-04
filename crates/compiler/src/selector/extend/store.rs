//! Registering style rules' selectors and `@extend`s, and extending those already registered.

use super::*;

impl ExtensionStore {
    /// Adds `selector` to this extender.
    ///
    /// Extends `selector` using any registered extensions, then returns the resulting
    /// selector. If any more relevant extensions are added, the returned selector
    /// is automatically updated.
    ///
    /// The `media_query_context` is the media query context in which the selector was
    /// defined, or `None` if it was defined at the top level of the document.
    pub fn add_selector(
        &mut self,
        mut selector: SelectorList,
        // span: Span,
        media_query_context: &Option<Vec<CssMediaQuery>>,
    ) -> ExtendedSelector {
        if !selector.is_invisible() {
            for complex in selector.components.clone() {
                self.originals.insert(&complex);
            }
        }

        if !self.extensions.is_empty() {
            selector = self.extend_list(selector, None, media_query_context);
            /*
              todo: when we have error handling
                  } on SassException catch (error) {
              throw SassException(
                  "From ${error.span.message('')}\n"
                  "${error.message}",
                  span);
            }
              */
        }
        let extended_selector = ExtendedSelector::new(selector.clone());
        if let Some(media_query_context) = media_query_context {
            self.media_contexts
                .insert(extended_selector.clone(), media_query_context.clone());
        }
        self.register_selector(selector, &extended_selector);
        extended_selector
    }

    /// Registers the `SimpleSelector`s in `list` to point to `selector` in
    /// `self.selectors`.
    pub(super) fn register_selector(&mut self, list: SelectorList, selector: &ExtendedSelector) {
        for complex in list.components {
            for component in complex.components {
                if let ComplexSelectorComponent::Compound(component) = component {
                    for simple in component.components {
                        // PERF: we compute the hash twice, which isn't great, but we avoid a superfluous
                        // clone in cases where we have already seen a simple selector (common in
                        // scenarios in which there is a lot of nesting)
                        if let Some(entry) = self.selectors.get_mut(&simple) {
                            entry.insert(selector.clone());
                        } else {
                            self.selectors
                                .entry(simple.clone())
                                .or_insert_with(SelectorHashSet::new)
                                .insert(selector.clone());
                        }

                        if let SimpleSelector::Pseudo(Pseudo {
                            selector: Some(simple_selector),
                            ..
                        }) = simple
                        {
                            self.register_selector(*simple_selector, selector);
                        }
                    }
                }
            }
        }
    }

    /// The mandatory extensions whose targets `callback` accepts, unmerged.
    pub fn extensions_where_target(
        &self,
        callback: impl Fn(&SimpleSelector) -> bool,
    ) -> Vec<Extension> {
        let mut result = Vec::new();
        for (target, sources) in &self.extensions {
            if !callback(target) {
                continue;
            }
            for extension in sources.values() {
                result.extend(
                    extension
                        .unmerge()
                        .into_iter()
                        .filter(|extension| !extension.is_optional),
                );
            }
        }
        result
    }

    /// Extends this store's selectors (and extensions) with the extensions of `extension_stores`,
    /// which are downstream of this one's module.
    pub fn add_extensions(&mut self, extension_stores: &[&ExtensionStore]) -> SassResult<()> {
        // Extensions already in `self` whose extenders are extended by the new extensions, and
        // thus which need to be updated.
        let mut extensions_to_extend: Option<Vec<Extension>> = None;

        // Selectors that contain simple selectors that are extended by the new extensions, and
        // thus which need to be extended themselves.
        let mut selectors_to_extend: Option<SelectorHashSet> = None;

        // An extension map with the same structure as `self.extensions` that only includes
        // extensions from `extension_stores`.
        let mut new_extensions: Option<
            HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>,
        > = None;

        for extension_store in extension_stores {
            if extension_store.is_empty() {
                continue;
            }
            self.source_specificity.extend(
                extension_store
                    .source_specificity
                    .iter()
                    .map(|(simple, specificity)| (simple.clone(), *specificity)),
            );
            for (target, new_sources) in &extension_store.extensions {
                // Private selectors can't be extended across module boundaries.
                if let SimpleSelector::Placeholder(name) = target
                    && (name.starts_with('-') || name.starts_with('_'))
                {
                    continue;
                }

                // Find existing extensions to extend.
                let extensions_for_target = self.extensions_by_extender.get(target).cloned();
                if let Some(extensions_for_target) = &extensions_for_target {
                    extensions_to_extend
                        .get_or_insert_with(Vec::new)
                        .extend(extensions_for_target.iter().cloned());
                }

                // Find existing selectors to extend.
                let selectors_for_target = self.selectors.get(target).cloned();
                if let Some(selectors_for_target) = &selectors_for_target {
                    selectors_to_extend
                        .get_or_insert_with(SelectorHashSet::new)
                        .extend(selectors_for_target.clone());
                }

                let has_existing =
                    extensions_for_target.is_some() || selectors_for_target.is_some();

                // Add `new_sources` to `self.extensions`.
                match self.extensions.get_mut(target) {
                    Some(existing_sources) => {
                        for (extender, extension) in new_sources {
                            let extension = match existing_sources.get(extender) {
                                Some(existing) => {
                                    MergedExtension::merge(existing.clone(), extension.clone())?
                                }
                                None => extension.clone(),
                            };
                            existing_sources.insert(extender.clone(), extension.clone());

                            if has_existing {
                                new_extensions
                                    .get_or_insert_with(HashMap::new)
                                    .entry(target.clone())
                                    .or_insert_with(IndexMap::new)
                                    .insert(extender.clone(), extension);
                            }
                        }
                    }
                    None => {
                        self.extensions.insert(target.clone(), new_sources.clone());
                        if has_existing {
                            new_extensions
                                .get_or_insert_with(HashMap::new)
                                .insert(target.clone(), new_sources.clone());
                        }
                    }
                }
            }
        }

        if let Some(new_extensions) = new_extensions {
            // The return value only matters for extend loops, which can't cross modules.
            if let Some(extensions_to_extend) = extensions_to_extend {
                self.extend_existing_extensions(extensions_to_extend, &new_extensions)?;
            }

            if let Some(selectors_to_extend) = selectors_to_extend {
                self.extend_existing_selectors(selectors_to_extend, &new_extensions);
            }
        }

        Ok(())
    }

    pub fn add_extension(
        &mut self,
        extender: SelectorList,
        target: &SimpleSelector,
        extend: &ExtendRule,
        media_context: &Option<Vec<CssMediaQuery>>,
        span: Span,
    ) -> SassResult<()> {
        let selectors = self.selectors.get(target).cloned();
        let existing_extensions = self.extensions_by_extender.get(target).cloned();

        let mut new_extensions: Option<IndexMap<ComplexSelector, Extension>> = None;

        for complex in extender.components {
            let state = Extension {
                specificity: complex.max_specificity(),
                extender: complex.clone(),
                target: Some(target.clone()),
                span,
                media_context: media_context.clone(),
                is_optional: extend.is_optional,
                is_original: false,
                left: None,
                right: None,
            };

            let sources = self
                .extensions
                .entry(target.clone())
                .or_insert_with(IndexMap::new);

            if let Some(existing_state) = sources.get(&complex).cloned() {
                // If there's already an extend from `extender` to `target`, we don't need
                // to re-run the extension. We may need to mark the extension as
                // mandatory, though.
                sources.insert(complex, MergedExtension::merge(existing_state, state)?);
                continue;
            }

            sources.insert(complex.clone(), state.clone());

            for simple in simple_selectors(&complex) {
                self.extensions_by_extender
                    .entry(simple.clone())
                    .or_insert_with(Vec::new)
                    .push(state.clone());
                // Only source specificity for the original selector is relevant.
                // Selectors generated by `@extend` don't get new specificity.
                self.source_specificity
                    .entry(simple)
                    .or_insert_with(|| complex.max_specificity());
            }

            if selectors.is_some() || existing_extensions.is_some() {
                new_extensions
                    .get_or_insert_with(IndexMap::new)
                    .insert(complex.clone(), state.clone());
            }
        }

        let new_extensions = if let Some(new) = new_extensions {
            new
        } else {
            return Ok(());
        };

        let mut new_extensions_by_target = HashMap::new();
        new_extensions_by_target.insert(target.clone(), new_extensions);

        if let Some(existing_extensions) = existing_extensions {
            let additional_extensions =
                self.extend_existing_extensions(existing_extensions, &new_extensions_by_target)?;
            if let Some(additional_extensions) = additional_extensions {
                map_add_all_2(&mut new_extensions_by_target, additional_extensions);
            }
        }

        if let Some(selectors) = selectors {
            self.extend_existing_selectors(selectors, &new_extensions_by_target);
        }
        Ok(())
    }

    /// Extend `extensions` using `new_extensions`.
    ///
    /// Note that this does duplicate some work done by
    /// `Extender::extend_existing_selectors`, but it's necessary to expand each extension's
    /// extender separately without reference to the full selector list, so that
    /// relevant results don't get trimmed too early.
    ///
    /// Returns extensions that should be added to `new_extensions` before
    /// extending selectors in order to properly handle extension loops such as:
    ///```foo
    ///     .c {x: y; @extend .a}
    ///     .x.y.a {@extend .b}
    ///     .z.b {@extend .c}
    ///```
    /// Returns `None` if there are no extensions to add.
    pub(super) fn extend_existing_extensions(
        &mut self,
        extensions: Vec<Extension>,
        new_extensions: &HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>,
    ) -> SassResult<Option<HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>>> {
        let mut additional_extensions: Option<
            HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>,
        > = None;
        for extension in extensions {
            let target = extension.target.clone().unwrap();
            // dart-sass updates this map in place; it is taken out of the store while the
            // extension is expanded, and put back below.
            let mut sources = self.extensions.get(&target).unwrap().clone();

            // `extend_existing_selectors` would have thrown already.
            let selectors: Vec<ComplexSelector> = if let Some(v) = self.extend_complex(
                extension.extender.clone(),
                Some(new_extensions),
                &extension.media_context,
            ) {
                v
            } else {
                continue;
            };
            // todo: when we add error handling, this error is special
            /*
            } on SassException catch (error) {
                throw SassException(
                    "From ${extension.extenderSpan.message('')}\n"
                    "${error.message}",
                    error.span);
            }
            */

            let contains_extension = selectors.first() == Some(&extension.extender);

            let mut first = false;
            for complex in selectors {
                // If the output contains the original complex selector, there's no
                // need to recreate it.
                if contains_extension && first {
                    first = false;
                    continue;
                }

                let with_extender = extension.clone().with_extender(complex.clone());
                if let Some(existing_extension) = sources.get(&complex).cloned() {
                    sources.insert(
                        complex.clone(),
                        MergedExtension::merge(existing_extension, with_extender)?,
                    );
                } else {
                    sources.insert(complex.clone(), with_extender.clone());

                    for component in complex.components.clone() {
                        if let ComplexSelectorComponent::Compound(component) = component {
                            for simple in component.components {
                                self.extensions_by_extender
                                    .entry(simple)
                                    .or_insert_with(Vec::new)
                                    .push(with_extender.clone());
                            }
                        }
                    }

                    if new_extensions.contains_key(&target) {
                        additional_extensions
                            .get_or_insert_with(HashMap::new)
                            .entry(target.clone())
                            .or_insert_with(IndexMap::new)
                            .insert(complex.clone(), with_extender.clone());
                    }
                }
            }
            // If `selectors` doesn't contain `extension.extender`, for example if it
            // was replaced due to :not() expansion, we must get rid of the old
            // version.
            if !contains_extension {
                // todo: evaluate whether we could get away with swap_remove
                sources.shift_remove(&extension.extender);
            }
            self.extensions.insert(target, sources);
        }
        Ok(additional_extensions)
    }

    /// Extend `extensions` using `new_extensions`.
    pub(super) fn extend_existing_selectors(
        &mut self,
        selectors: SelectorHashSet,
        new_extensions: &HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>,
    ) {
        for mut selector in selectors {
            let old_value = selector.clone().into_selector().0;
            selector.set_inner(self.extend_list(
                old_value.clone(),
                Some(new_extensions),
                &self.media_contexts.get(&selector).cloned(),
            ));
            /*
            todo: error handling
            } on SassException catch (error) {
            throw SassException(
                "From ${selector.span.message('')}\n"
                "${error.message}",
                error.span);
            }

            */

            // If no extends actually happened (for example because unification
            // failed), we don't need to re-register the selector.
            let selector_as_selector = selector.clone().into_selector().0;
            if old_value == selector_as_selector {
                continue;
            }
            self.register_selector(selector_as_selector, &selector);
        }
    }
}
