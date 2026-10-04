//! Making the members of imported and forwarded modules available, and the configuration an import passes on.

use super::*;

impl Environment {
    /// Makes the members forwarded by [module] available in the current
    /// environment.
    ///
    /// This is called when [module] is `@import`ed.
    pub fn import_forwards(&mut self, _env: Module) {
        if let Module::Environment { env, .. } = _env {
            let mut forwarded = env.forwarded_modules;

            if (*forwarded).borrow().is_empty() {
                return;
            }

            // Omit modules from [forwarded] that are already globally available and
            // forwarded in this module.
            let forwarded_modules = Arc::clone(&self.forwarded_modules);
            if !(*forwarded_modules).borrow().is_empty() {
                // todo: intermediate name
                let mut x = Vec::new();
                for entry in (*forwarded).borrow().iter() {
                    if !forwarded_modules
                        .borrow()
                        .iter()
                        .any(|module| Arc::ptr_eq(module, entry))
                        || !self
                            .global_modules
                            .iter()
                            .any(|module| Arc::ptr_eq(module, entry))
                    {
                        x.push(Arc::clone(entry));
                    }
                }

                forwarded = Arc::new(RefCell::new(x));
            }

            let forwarded_var_names = forwarded
                .borrow()
                .iter()
                .flat_map(|module| (*module).borrow().scope().variables.keys())
                .collect::<IndexSet<Identifier>>();
            let forwarded_fn_names = forwarded
                .borrow()
                .iter()
                .flat_map(|module| (*module).borrow().scope().functions.keys())
                .collect::<IndexSet<Identifier>>();
            let forwarded_mixin_names = forwarded
                .borrow()
                .iter()
                .flat_map(|module| (*module).borrow().scope().mixins.keys())
                .collect::<IndexSet<Identifier>>();

            if self.at_root() {
                let mut to_remove = Vec::new();

                // Hide members from modules that have already been imported or
                // forwarded that would otherwise conflict with the @imported members.
                for (idx, module) in (*self.imported_modules).borrow().iter().enumerate() {
                    let shadowed = ShadowedModule::if_necessary(
                        Arc::clone(module),
                        Some(&forwarded_var_names),
                        Some(&forwarded_fn_names),
                        Some(&forwarded_mixin_names),
                    );

                    if shadowed.is_some() {
                        to_remove.push(idx);
                    }
                }

                let mut imported_modules = (*self.imported_modules).borrow_mut();

                for &idx in to_remove.iter().rev() {
                    imported_modules.remove(idx);
                }

                to_remove.clear();

                for (idx, module) in (*self.forwarded_modules).borrow().iter().enumerate() {
                    let shadowed = ShadowedModule::if_necessary(
                        Arc::clone(module),
                        Some(&forwarded_var_names),
                        Some(&forwarded_fn_names),
                        Some(&forwarded_mixin_names),
                    );

                    if shadowed.is_some() {
                        to_remove.push(idx);
                    }
                }

                let mut forwarded_modules = (*self.forwarded_modules).borrow_mut();

                for &idx in to_remove.iter().rev() {
                    forwarded_modules.remove(idx);
                }

                imported_modules.extend(forwarded.borrow().iter().map(Arc::clone));
                forwarded_modules.extend(forwarded.borrow().iter().map(Arc::clone));
            } else {
                self.nested_forwarded_modules
                    .get_or_insert_with(|| {
                        Arc::new(RefCell::new(
                            (0..self.scopes.len())
                                .map(|_| Arc::new(RefCell::new(Vec::new())))
                                .collect(),
                        ))
                    })
                    .borrow_mut()
                    .last_mut()
                    .unwrap()
                    .borrow_mut()
                    .extend(forwarded.borrow().iter().map(Arc::clone));
            }

            // Remove existing member definitions that are now shadowed by the
            // forwarded modules.
            for variable in forwarded_var_names {
                (*self.scopes.variables)
                    .borrow_mut()
                    .last_mut()
                    .unwrap()
                    .borrow_mut()
                    .shift_remove(&variable);
            }
            self.scopes.last_variable_index = None;

            for func in forwarded_fn_names {
                (*self.scopes.functions)
                    .borrow_mut()
                    .last_mut()
                    .unwrap()
                    .borrow_mut()
                    .shift_remove(&func);
            }
            for mixin in forwarded_mixin_names {
                (*self.scopes.mixins)
                    .borrow_mut()
                    .last_mut()
                    .unwrap()
                    .borrow_mut()
                    .shift_remove(&mixin);
            }
        }
    }

    pub fn to_implicit_configuration(&self) -> Configuration {
        let mut configuration = IndexMap::new();

        let variables = (*self.scopes.variables).borrow();

        for variables in variables.iter() {
            let entries = (**variables).borrow();
            for (key, value) in entries.iter() {
                // Implicit configurations are never invalid, making [configurationSpan]
                // unnecessary, so we pass null here to avoid having to compute it.
                configuration.insert(*key, ConfiguredValue::implicit(value.clone()));
            }
        }

        Configuration::implicit(configuration)
    }
}
