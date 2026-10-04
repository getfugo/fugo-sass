//! Modules: `@use`, `@forward` and their configurations.

use super::*;

impl<'a> Visitor<'a> {
    pub(super) fn visit_forward_rule(&mut self, forward_rule: AstForwardRule) -> SassResult<()> {
        let old_config = Rc::clone(&self.configuration);
        let adjusted_config = Configuration::through_forward(Rc::clone(&old_config), &forward_rule);

        if !forward_rule.configuration.is_empty() {
            let new_configuration =
                self.add_forward_configuration(Rc::clone(&adjusted_config), &forward_rule)?;

            self.load_module(
                forward_rule.url.as_path(),
                Some(Rc::clone(&new_configuration)),
                false,
                forward_rule.span,
                |visitor, module, _| {
                    visitor.env.forward_module(module, forward_rule.clone());

                    Ok(())
                },
            )?;

            Self::remove_used_configuration(
                &adjusted_config,
                &new_configuration,
                &forward_rule
                    .configuration
                    .iter()
                    .filter(|var| !var.is_guarded)
                    .map(|var| var.name.node)
                    .collect(),
            );

            // Remove all the variables that weren't configured by this particular
            // `@forward` before checking that the configuration is empty. Errors for
            // outer `with` clauses will be thrown once those clauses finish
            // executing.
            let configured_variables: IndexSet<Identifier> = forward_rule
                .configuration
                .iter()
                .map(|var| var.name.node)
                .collect();

            let mut to_remove = Vec::new();

            for name in (*new_configuration).borrow().values.keys() {
                if !configured_variables.contains(&name) {
                    to_remove.push(name);
                }
            }

            for name in to_remove {
                (*new_configuration).borrow_mut().remove(name);
            }

            Self::assert_configuration_is_empty(&new_configuration, false)?;
        } else {
            self.configuration = adjusted_config;
            let url = forward_rule.url.clone();
            self.load_module(
                url.as_path(),
                None,
                false,
                forward_rule.span,
                move |visitor, module, _| {
                    visitor.env.forward_module(module, forward_rule.clone());

                    Ok(())
                },
            )?;
            self.configuration = old_config;
        }

        Ok(())
    }

    #[allow(clippy::unnecessary_unwrap)]
    pub(super) fn add_forward_configuration(
        &mut self,
        config: Rc<RefCell<Configuration>>,
        forward_rule: &AstForwardRule,
    ) -> SassResult<Rc<RefCell<Configuration>>> {
        let mut new_values = IndexMap::from_iter((*config).borrow().values.iter());

        for variable in &forward_rule.configuration {
            if variable.is_guarded {
                let old_value = (*config).borrow_mut().remove(variable.name.node);

                if old_value.is_some()
                    && !matches!(
                        old_value,
                        Some(ConfiguredValue {
                            value: Value::Null,
                            ..
                        })
                    )
                {
                    new_values.insert(variable.name.node, old_value.unwrap());
                    continue;
                }
            }

            // todo: superfluous clone?
            let value = self.visit_expr(variable.expr.node.clone())?;
            let value = self.without_slash(value);

            new_values.insert(
                variable.name.node,
                ConfiguredValue::explicit(value, variable.expr.span),
            );
        }

        Ok(Rc::new(RefCell::new(
            if !(*config).borrow().is_implicit() || (*config).borrow().is_empty() {
                Configuration::explicit(new_values, forward_rule.span)
            } else {
                Configuration::implicit(new_values)
            },
        )))
    }

    /// Remove configured values from [upstream] that have been removed from
    /// [downstream], unless they match a name in [except].
    pub(super) fn remove_used_configuration(
        upstream: &Rc<RefCell<Configuration>>,
        downstream: &Rc<RefCell<Configuration>>,
        except: &IndexSet<Identifier>,
    ) {
        let mut names_to_remove = Vec::new();
        let downstream_keys = (*downstream).borrow().values.keys();
        for name in (*upstream).borrow().values.keys() {
            if except.contains(&name) {
                continue;
            }

            if !downstream_keys.contains(&name) {
                names_to_remove.push(name);
            }
        }

        for name in names_to_remove {
            (*upstream).borrow_mut().remove(name);
        }
    }

    pub(super) fn execute(
        &mut self,
        stylesheet: StyleSheet,
        configuration: Option<Rc<RefCell<Configuration>>>,
        // todo: different errors based on this
        _names_in_errors: bool,
    ) -> SassResult<Arc<RefCell<Module>>> {
        let url = stylesheet.url.clone();

        // todo: use canonical url for modules
        if let Some(already_loaded) = self.modules.get(&stylesheet.url) {
            let current_configuration =
                configuration.unwrap_or_else(|| Rc::clone(&self.configuration));

            if !current_configuration.borrow().is_implicit() {
                //   if (!_moduleConfigurations[url]!.sameOriginal(currentConfiguration) &&
                //       currentConfiguration is ExplicitConfiguration) {
                //     var message = namesInErrors
                //         ? "${p.prettyUri(url)} was already loaded, so it can't be "
                //             "configured using \"with\"."
                //         : "This module was already loaded, so it can't be configured using "
                //             "\"with\".";

                //     var existingSpan = _moduleNodes[url]?.span;
                //     var configurationSpan = configuration == null
                //         ? currentConfiguration.nodeWithSpan.span
                //         : null;
                //     var secondarySpans = {
                //       if (existingSpan != null) existingSpan: "original load",
                //       if (configurationSpan != null) configurationSpan: "configuration"
                //     };

                //     throw secondarySpans.isEmpty
                //         ? _exception(message)
                //         : _multiSpanException(message, "new load", secondarySpans);
                //   }
            }

            return Ok(Arc::clone(already_loaded));
        }

        let env = Environment::new();
        let mut extension_store = ExtensionStore::new(self.empty_span);
        let css_before = self.css_tree.root_len() + self.import_nodes.len();
        let outer_nested_module_css = mem::take(&mut self.nested_module_css);

        self.with_environment::<SassResult<()>, _>(env.new_closure(), |visitor| {
            let old_parent = visitor.parent;
            mem::swap(&mut visitor.extender, &mut extension_store);
            let old_style_rule = visitor.style_rule_ignoring_at_root.take();
            let old_style_rule_from_plain_css = mem::take(&mut visitor.style_rule_from_plain_css);
            let old_style_rule_idx = visitor.style_rule_idx.take();
            let old_media_queries = visitor.media_queries.take();
            let old_declaration_name = visitor.declaration_name.take();
            let old_in_unknown_at_rule = visitor.flags.in_unknown_at_rule();
            let old_at_root_excluding_style_rule = visitor.flags.at_root_excluding_style_rule();
            let old_in_keyframes = visitor.flags.in_keyframes();
            let old_configuration = if let Some(new_config) = configuration {
                Some(mem::replace(&mut visitor.configuration, new_config))
            } else {
                None
            };
            visitor.parent = None;
            visitor.flags.set(ContextFlags::IN_UNKNOWN_AT_RULE, false);
            visitor
                .flags
                .set(ContextFlags::AT_ROOT_EXCLUDING_STYLE_RULE, false);
            visitor.flags.set(ContextFlags::IN_KEYFRAMES, false);

            visitor.visit_stylesheet(stylesheet)?;

            // visitor.importer = old_importer;
            // visitor.stylesheet = old_stylesheet;
            // visitor.root = old_root;
            visitor.parent = old_parent;
            // visitor.end_of_imports = old_end_of_imports;
            // visitor.out_of_order_imports = old_out_of_order_imports;
            mem::swap(&mut visitor.extender, &mut extension_store);
            visitor.style_rule_ignoring_at_root = old_style_rule;
            visitor.style_rule_from_plain_css = old_style_rule_from_plain_css;
            visitor.style_rule_idx = old_style_rule_idx;
            visitor.media_queries = old_media_queries;
            visitor.declaration_name = old_declaration_name;
            visitor
                .flags
                .set(ContextFlags::IN_UNKNOWN_AT_RULE, old_in_unknown_at_rule);
            visitor.flags.set(
                ContextFlags::AT_ROOT_EXCLUDING_STYLE_RULE,
                old_at_root_excluding_style_rule,
            );
            visitor
                .flags
                .set(ContextFlags::IN_KEYFRAMES, old_in_keyframes);
            if let Some(old_config) = old_configuration {
                visitor.configuration = old_config;
            }

            Ok(())
        })?;

        let css_written = self.css_tree.root_len() + self.import_nodes.len() - css_before;
        let contains_css = css_written > self.nested_module_css;
        self.nested_module_css = outer_nested_module_css + css_written;

        let module = env.to_module(extension_store, contains_css);

        self.modules.insert(url, Arc::clone(&module));

        Ok(module)
    }

    pub(crate) fn load_module(
        &mut self,
        url: &Path,
        configuration: Option<Rc<RefCell<Configuration>>>,
        names_in_errors: bool,
        span: Span,
        callback: impl Fn(&mut Self, Arc<RefCell<Module>>, StyleSheet) -> SassResult<()>,
    ) -> SassResult<()> {
        let builtin = match url.to_string_lossy().as_ref() {
            "sass:color" => Some(declare_module_color()),
            "sass:list" => Some(declare_module_list()),
            "sass:map" => Some(declare_module_map()),
            "sass:math" => Some(declare_module_math()),
            "sass:meta" => Some(declare_module_meta()),
            "sass:selector" => Some(declare_module_selector()),
            "sass:string" => Some(declare_module_string()),
            _ => None,
        };

        if let Some(builtin) = builtin {
            if let Some(configuration) = configuration.as_ref().map(|c| c.borrow())
                && !configuration.is_implicit()
            {
                let msg = if names_in_errors {
                    format!(
                        "Built-in module {} can't be configured.",
                        url.to_string_lossy()
                    )
                } else {
                    "Built-in modules can't be configured.".to_owned()
                };

                return Err((msg, configuration.span.unwrap()).into());
            }

            callback(
                self,
                Arc::new(RefCell::new(builtin)),
                StyleSheet::new(false, url.to_path_buf()),
            )?;
            return Ok(());
        }

        // todo: decide on naming convention for style_sheet vs stylesheet
        let stylesheet = self.load_style_sheet(url.to_string_lossy().as_ref(), false, span)?;

        let canonical_url = self
            .options
            .fs
            .canonicalize(&stylesheet.url)
            .unwrap_or_else(|_| stylesheet.url.clone());

        if self.active_modules.contains(&canonical_url) {
            return Err(("Module loop: this module is already being loaded.", span).into());
        }

        self.active_modules.insert(canonical_url.clone());

        let module = self.execute(stylesheet.clone(), configuration, names_in_errors)?;

        self.active_modules.remove(&canonical_url);

        callback(self, module, stylesheet)?;

        Ok(())
    }

    pub(super) fn visit_use_rule(&mut self, use_rule: AstUseRule) -> SassResult<()> {
        let configuration = if use_rule.configuration.is_empty() {
            Rc::new(RefCell::new(Configuration::empty()))
        } else {
            let mut values = IndexMap::new();

            for var in use_rule.configuration {
                let value = self.visit_expr(var.expr.node)?;
                let value = self.without_slash(value);
                values.insert(
                    var.name.node,
                    ConfiguredValue::explicit(value, var.name.span.merge(var.expr.span)),
                );
            }

            Rc::new(RefCell::new(Configuration::explicit(values, use_rule.span)))
        };

        let span = use_rule.span;

        let namespace = use_rule
            .namespace
            .as_ref()
            .map(|s| Identifier::namespace(s.trim_start_matches("sass:")));

        self.load_module(
            &use_rule.url,
            Some(Rc::clone(&configuration)),
            false,
            span,
            |visitor, module, _| {
                visitor.env.add_module(namespace, module, span)?;

                Ok(())
            },
        )?;

        Self::assert_configuration_is_empty(&configuration, false)?;

        Ok(())
    }

    pub(crate) fn assert_configuration_is_empty(
        config: &Rc<RefCell<Configuration>>,
        name_in_error: bool,
    ) -> SassResult<()> {
        let config = (**config).borrow();
        // By definition, implicit configurations are allowed to only use a subset
        // of their values.
        if config.is_empty() || config.is_implicit() {
            return Ok(());
        }

        let Spanned { node: name, span } = config.first().unwrap();

        let msg = if name_in_error {
            format!(
                "${name} was not declared with !default in the @used module.",
                name = name
            )
        } else {
            "This variable was not declared with !default in the @used module.".to_owned()
        };

        Err((msg, span).into())
    }
}
