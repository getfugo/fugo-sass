//! `@extend`, and extending the modules upstream of a document.

use super::*;

/// Whether `module` or a module upstream of it wrote CSS.
pub(super) fn transitively_contains_css(
    module: &Arc<RefCell<Module>>,
    memo: &mut HashMap<*const RefCell<Module>, bool>,
) -> bool {
    let key = Arc::as_ptr(module);
    if let Some(&contains_css) = memo.get(&key) {
        return contains_css;
    }
    let result = match &*module.borrow() {
        Module::Environment {
            contains_css,
            upstream,
            ..
        } => {
            *contains_css
                || upstream
                    .iter()
                    .any(|upstream| transitively_contains_css(upstream, memo))
        }
        _ => false,
    };
    memo.insert(key, result);
    result
}

/// The modules upstream of a document using `upstream`, downstream ones first (dart-sass's
/// `_topologicalModules`, without the root).
pub(super) fn topological_modules(upstream: &[Arc<RefCell<Module>>]) -> Vec<Arc<RefCell<Module>>> {
    pub(super) fn visit(
        module: &Arc<RefCell<Module>>,
        seen: &mut HashSet<*const RefCell<Module>>,
        sorted: &mut VecDeque<Arc<RefCell<Module>>>,
    ) {
        if let Module::Environment { upstream, .. } = &*module.borrow() {
            for upstream in upstream {
                if seen.insert(Arc::as_ptr(upstream)) {
                    visit(upstream, seen, sorted);
                }
            }
        }
        sorted.push_front(Arc::clone(module));
    }

    let mut seen = HashSet::new();
    let mut sorted = VecDeque::new();
    for module in upstream {
        if seen.insert(Arc::as_ptr(module)) {
            visit(module, &mut seen, &mut sorted);
        }
    }
    sorted.into()
}

/// The error for a mandatory `@extend` whose target is in no style rule.
pub(super) fn unsatisfied_extension(extension: &Extension) -> Box<SassError> {
    (
        format!(
            "The target selector was not found.\nUse \"@extend {} !optional\" to avoid this error.",
            extension.target.as_ref().unwrap()
        ),
        extension.span,
    )
        .into()
}

impl<'a> Visitor<'a> {
    /// Applies each module's `@extend`s to the modules upstream of it, and errors for a mandatory
    /// `@extend` whose target no module has (the extension part of dart-sass's `_combineCss` and
    /// `_extendModules`).
    pub(super) fn extend_modules(&mut self) -> SassResult<()> {
        let root_upstream = self.env.all_modules.borrow().clone();

        let mut contains_css = HashMap::new();
        if !root_upstream
            .iter()
            .any(|module| transitively_contains_css(module, &mut contains_css))
        {
            let selectors = self.extender.simple_selectors();
            if let Some(extension) = self
                .extender
                .extensions_where_target(|target| !selectors.contains(target))
                .into_iter()
                .next()
            {
                return Err(unsatisfied_extension(&extension));
            }
            return Ok(());
        }

        // Downstream modules first; the root module (this document) is the first of all.
        let sorted = topological_modules(&root_upstream);

        // The extension stores directly downstream of each module, by the module's address.
        let mut downstream_stores: HashMap<*const RefCell<Module>, Vec<Arc<RefCell<Module>>>> =
            HashMap::new();
        // Whether the root's store is downstream of each module.
        let mut downstream_of_root: HashSet<*const RefCell<Module>> = HashSet::new();
        // Extensions that no upstream module has satisfied yet.
        let mut unsatisfied = Vec::new();

        let original_selectors = self.extender.simple_selectors();
        unsatisfied.extend(
            self.extender
                .extensions_where_target(|target| !original_selectors.contains(target)),
        );
        if !self.extender.is_empty() {
            for upstream in &root_upstream {
                downstream_of_root.insert(Arc::as_ptr(upstream));
            }
            unsatisfied.retain(|extension: &Extension| {
                !original_selectors.contains(extension.target.as_ref().unwrap())
            });
        }

        for module in sorted {
            let key = Arc::as_ptr(&module);
            let downstream = downstream_stores.remove(&key).unwrap_or_default();
            let downstream_guards: Vec<_> = downstream.iter().map(|m| m.borrow()).collect();
            let mut stores: Vec<&ExtensionStore> = downstream_guards
                .iter()
                .filter_map(|module| match &**module {
                    Module::Environment {
                        extension_store, ..
                    } => Some(extension_store),
                    _ => None,
                })
                .collect();
            if downstream_of_root.contains(&key) {
                stores.push(&self.extender);
            }

            let mut module_ref = module.borrow_mut();
            let (extension_store, upstream) = match &mut *module_ref {
                Module::Environment {
                    extension_store,
                    upstream,
                    ..
                } => (extension_store, upstream),
                _ => continue,
            };

            let original_selectors = extension_store.simple_selectors();
            unsatisfied.extend(
                extension_store
                    .extensions_where_target(|target| !original_selectors.contains(target)),
            );

            if !stores.is_empty() {
                extension_store.add_extensions(&stores)?;
            }
            if extension_store.is_empty() {
                continue;
            }

            for upstream in upstream.iter() {
                downstream_stores
                    .entry(Arc::as_ptr(upstream))
                    .or_default()
                    .push(Arc::clone(&module));
            }

            unsatisfied.retain(|extension: &Extension| {
                !original_selectors.contains(extension.target.as_ref().unwrap())
            });
        }

        match unsatisfied.first() {
            Some(extension) => Err(unsatisfied_extension(extension)),
            None => Ok(()),
        }
    }

    pub(crate) fn parse_selector_from_string(
        &mut self,
        selector_text: &str,
        allows_parent: bool,
        allows_placeholder: bool,
        span: Span,
    ) -> SassResult<SelectorList> {
        let sel_toks = Lexer::new_from_string(selector_text, span);

        SelectorParser::new(sel_toks, allows_parent, allows_placeholder, span).parse()
    }

    pub(super) fn visit_extend_rule(
        &mut self,
        extend_rule: AstExtendRule,
    ) -> SassResult<Option<Value>> {
        if !self.style_rule_exists() || self.declaration_name.is_some() {
            return Err((
                "@extend may only be used within style rules.",
                extend_rule.span,
            )
                .into());
        }

        let super_selector = self.style_rule_ignoring_at_root.clone().unwrap();

        let target_text = self.interpolation_to_value(extend_rule.value, false, true)?;

        let list = self.parse_selector_from_string(&target_text, false, true, extend_rule.span)?;

        for complex in list.components {
            if complex.components.len() != 1 || !complex.components.first().unwrap().is_compound() {
                // If the selector was a compound selector but not a simple
                // selector, emit a more explicit error.
                return Err(("complex selectors may not be extended.", extend_rule.span).into());
            }

            let compound = match complex.components.first() {
                Some(ComplexSelectorComponent::Compound(c)) => c,
                Some(..) | None => unreachable!("checked by above condition"),
            };
            if compound.components.len() != 1 {
                return Err((
                    format!(
                        "compound selectors may no longer be extended.\nConsider `@extend {}` instead.\nSee http://bit.ly/ExtendCompound for details.\n",
                        compound.components.iter().map(ToString::to_string).collect::<Vec<String>>().join(", ")
                    )
                , extend_rule.span).into());
            }

            self.extender.add_extension(
                super_selector.clone().into_selector().0,
                compound.components.first().unwrap(),
                &ExtendRule {
                    is_optional: extend_rule.is_optional,
                },
                &self.media_queries,
                extend_rule.span,
            )?;
        }

        Ok(None)
    }
}
