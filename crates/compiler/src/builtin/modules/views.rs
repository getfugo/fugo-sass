//! Views of modules: forwarded with a prefix or show and hide lists, and shadowed by an import.

use super::*;

/// A [Module] that only exposes members that aren't shadowed by a given
/// blocklist of member names.
#[derive(Debug, Clone)]
pub(crate) struct ShadowedModule {
    #[allow(dead_code)]
    inner: Arc<RefCell<Module>>,
    pub(super) scope: ModuleScope,
}

impl ShadowedModule {
    pub fn new(
        module: Arc<RefCell<Module>>,
        variables: Option<&IndexSet<Identifier>>,
        functions: Option<&IndexSet<Identifier>>,
        mixins: Option<&IndexSet<Identifier>>,
    ) -> Self {
        let module_scope = module.borrow().scope();

        let variables = Self::shadowed_map(Arc::clone(&module_scope.variables), variables);
        let functions = Self::shadowed_map(Arc::clone(&module_scope.functions), functions);
        let mixins = Self::shadowed_map(Arc::clone(&module_scope.mixins), mixins);

        let new_scope = ModuleScope {
            variables,
            functions,
            mixins,
        };

        Self {
            inner: module,
            scope: new_scope,
        }
    }

    fn needs_blocklist<V: fmt::Debug + Clone>(
        map: Arc<dyn MapView<Value = V>>,
        blocklist: Option<&IndexSet<Identifier>>,
    ) -> bool {
        blocklist.is_some()
            && !map.is_empty()
            && blocklist.unwrap().iter().any(|key| map.contains_key(*key))
    }

    fn shadowed_map<V: fmt::Debug + Clone + 'static>(
        map: Arc<dyn MapView<Value = V>>,
        blocklist: Option<&IndexSet<Identifier>>,
    ) -> Arc<dyn MapView<Value = V>> {
        match blocklist {
            Some(..) if !Self::needs_blocklist(Arc::clone(&map), blocklist) => map,
            Some(blocklist) => Arc::new(LimitedMapView::blocklist(map, blocklist)),
            None => map,
        }
    }

    pub fn if_necessary(
        module: Arc<RefCell<Module>>,
        variables: Option<&IndexSet<Identifier>>,
        functions: Option<&IndexSet<Identifier>>,
        mixins: Option<&IndexSet<Identifier>>,
    ) -> Option<Arc<RefCell<Module>>> {
        let module_scope = module.borrow().scope();

        let needs_blocklist = Self::needs_blocklist(Arc::clone(&module_scope.variables), variables)
            || Self::needs_blocklist(Arc::clone(&module_scope.functions), functions)
            || Self::needs_blocklist(Arc::clone(&module_scope.mixins), mixins);

        if needs_blocklist {
            Some(Arc::new(RefCell::new(Module::Shadowed(Self::new(
                module, variables, functions, mixins,
            )))))
        } else {
            None
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ForwardedModule {
    pub(super) scope: ModuleScope,
    #[allow(dead_code)]
    inner: Arc<RefCell<Module>>,
    #[allow(dead_code)]
    forward_rule: AstForwardRule,
}

impl ForwardedModule {
    pub fn new(module: Arc<RefCell<Module>>, rule: AstForwardRule) -> Self {
        let scope = (*module).borrow().scope();

        let variables = Self::forwarded_map(
            scope.variables,
            rule.prefix.as_deref(),
            rule.shown_variables.as_ref(),
            rule.hidden_variables.as_ref(),
        );

        let functions = Self::forwarded_map(
            scope.functions,
            rule.prefix.as_deref(),
            rule.shown_mixins_and_functions.as_ref(),
            rule.hidden_mixins_and_functions.as_ref(),
        );

        let mixins = Self::forwarded_map(
            scope.mixins,
            rule.prefix.as_deref(),
            rule.shown_mixins_and_functions.as_ref(),
            rule.hidden_mixins_and_functions.as_ref(),
        );

        let scope = ModuleScope {
            variables,
            mixins,
            functions,
        };

        ForwardedModule {
            inner: module,
            forward_rule: rule,
            scope,
        }
    }

    fn forwarded_map<T: Clone + fmt::Debug + 'static>(
        mut map: Arc<dyn MapView<Value = T>>,
        prefix: Option<&str>,
        safelist: Option<&IndexSet<Identifier>>,
        blocklist: Option<&IndexSet<Identifier>>,
    ) -> Arc<dyn MapView<Value = T>> {
        debug_assert!(safelist.is_none() || blocklist.is_none());

        let blocklist = blocklist.filter(|blocklist| !blocklist.is_empty());
        if prefix.is_none() && safelist.is_none() && blocklist.is_none() {
            return map;
        }

        if let Some(prefix) = prefix {
            map = Arc::new(PrefixedMapView(map, prefix.to_owned()));
        }

        if let Some(safelist) = safelist {
            map = Arc::new(LimitedMapView::safelist(map, safelist));
        } else if let Some(blocklist) = blocklist {
            map = Arc::new(LimitedMapView::blocklist(map, blocklist));
        }

        map
    }

    pub fn if_necessary(
        module: Arc<RefCell<Module>>,
        rule: AstForwardRule,
    ) -> Arc<RefCell<Module>> {
        if rule.prefix.is_none()
            && rule.shown_mixins_and_functions.is_none()
            && rule.shown_variables.is_none()
            && rule
                .hidden_mixins_and_functions
                .as_ref()
                .is_some_and(IndexSet::is_empty)
            && rule
                .hidden_variables
                .as_ref()
                .is_some_and(IndexSet::is_empty)
        {
            module
        } else {
            Arc::new(RefCell::new(Module::Forwarded(ForwardedModule::new(
                module, rule,
            ))))
        }
    }
}
