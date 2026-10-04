//! `@import`, and loading stylesheets.

use super::*;

impl<'a> Visitor<'a> {
    pub(super) fn visit_import_rule(
        &mut self,
        import_rule: AstImportRule,
    ) -> SassResult<Option<Value>> {
        for import in import_rule.imports {
            match import {
                AstImport::Sass(dynamic_import) => {
                    self.visit_dynamic_import_rule(&dynamic_import)?;
                }
                AstImport::Plain(static_import) => self.visit_static_import_rule(static_import)?,
            }
        }

        Ok(None)
    }

    /// Searches the current directory of the file then searches in `load_paths` directories
    /// if the import has not yet been found.
    ///
    /// <https://sass-lang.com/documentation/at-rules/import#finding-the-file>
    /// <https://sass-lang.com/documentation/at-rules/import#load-paths>
    #[allow(clippy::cognitive_complexity, clippy::redundant_clone)]
    pub fn find_import(&self, path: &Path) -> Option<PathBuf> {
        let path_buf = if path.is_absolute() {
            path.into()
        } else {
            self.current_import_path
                .parent()
                .unwrap_or_else(|| Path::new(""))
                .join(path)
        };

        macro_rules! try_path {
            ($path:expr) => {
                let path = $path;
                let dirname = path.parent().unwrap_or_else(|| Path::new(""));
                let basename = path.file_name().unwrap_or_else(|| OsStr::new(".."));

                let partial = dirname.join(format!("_{}", basename.to_str().unwrap()));

                if self.options.fs.is_file(&path) {
                    return Some(path.to_path_buf());
                }

                if self.options.fs.is_file(&partial) {
                    return Some(partial);
                }
            };
        }

        if path_buf.extension() == Some(OsStr::new("scss"))
            || path_buf.extension() == Some(OsStr::new("sass"))
            || path_buf.extension() == Some(OsStr::new("css"))
        {
            let extension = path_buf.extension().unwrap();
            try_path!(path_buf.with_extension(format!(".import{}", extension.to_str().unwrap())));
            try_path!(path_buf);
            // todo: consider load paths
            return None;
        }

        macro_rules! try_path_with_extensions {
            ($path:expr) => {
                let path = $path;
                try_path!(path.with_extension("import.sass"));
                try_path!(path.with_extension("import.scss"));
                try_path!(path.with_extension("import.css"));
                try_path!(path.with_extension("sass"));
                try_path!(path.with_extension("scss"));
                try_path!(path.with_extension("css"));
            };
        }

        try_path_with_extensions!(path_buf.clone());

        if self.options.fs.is_dir(&path_buf) {
            try_path_with_extensions!(path_buf.join("index"));
        }

        for load_path in &self.options.load_paths {
            let path_buf = load_path.join(path);

            try_path_with_extensions!(&path_buf);

            if self.options.fs.is_dir(&path_buf) {
                try_path_with_extensions!(path_buf.join("index"));
            }
        }

        None
    }

    pub(super) fn parse_file(
        &mut self,
        lexer: Lexer,
        path: &Path,
        empty_span: Span,
    ) -> SassResult<StyleSheet> {
        match InputSyntax::for_path(path) {
            InputSyntax::Scss => ScssParser::new(lexer, self.options, empty_span, path).__parse(),
            InputSyntax::Sass => SassParser::new(lexer, self.options, empty_span, path).__parse(),
            InputSyntax::Css => CssParser::new(lexer, self.options, empty_span, path).__parse(),
        }
    }

    pub(super) fn import_like_node(
        &mut self,
        url: &str,
        _for_import: bool,
        span: Span,
    ) -> SassResult<StyleSheet> {
        if let Some(name) = self.find_import(url.as_ref()) {
            let name = self.options.fs.canonicalize(&name).unwrap_or(name);
            if let Some(style_sheet) = self.import_cache.get(&name) {
                return Ok(style_sheet.clone());
            }

            let file = self.map.add_file(
                name.to_string_lossy().into(),
                String::from_utf8(self.options.fs.read(&name)?)?,
            );

            let old_is_use_allowed = self.flags.is_use_allowed();
            self.flags.set(ContextFlags::IS_USE_ALLOWED, true);

            let style_sheet =
                self.parse_file(Lexer::new_from_file(&file), &name, file.span.subspan(0, 0))?;

            self.flags
                .set(ContextFlags::IS_USE_ALLOWED, old_is_use_allowed);

            if self.files_seen.contains(&name) {
                self.import_cache.insert(name, style_sheet.clone());
            } else {
                self.files_seen.insert(name);
            }

            return Ok(style_sheet);
        }

        Err(("Can't find stylesheet to import.", span).into())
    }

    pub(crate) fn load_style_sheet(
        &mut self,
        url: &str,
        // default=false
        for_import: bool,
        span: Span,
    ) -> SassResult<StyleSheet> {
        // todo: import cache
        self.import_like_node(url, for_import, span)
    }

    pub(super) fn visit_dynamic_import_rule(
        &mut self,
        dynamic_import: &AstSassImport,
    ) -> SassResult<()> {
        let stylesheet = self.load_style_sheet(&dynamic_import.url, true, dynamic_import.span)?;

        let url = stylesheet.url.clone();

        if self.active_modules.contains(&url) {
            return Err(("This file is already being loaded.", dynamic_import.span).into());
        }

        self.active_modules.insert(url.clone());

        // If the imported stylesheet doesn't use any modules, we can inject its
        // CSS directly into the current stylesheet. If it does use modules, we
        // need to put its CSS into an intermediate [ModifiableCssStylesheet] so
        // that we can hermetically resolve `@extend`s before injecting it.
        if stylesheet.uses.is_empty() && stylesheet.forwards.is_empty() {
            self.visit_stylesheet(stylesheet)?;
            return Ok(());
        }

        // todo:
        let loads_user_defined_modules = true;

        // this todo should be unreachable, as we currently do not push
        // to stylesheet.uses or stylesheet.forwards
        // let mut children = Vec::new();
        let env = self.env.for_import();

        self.with_environment::<SassResult<()>, _>(env.clone(), |visitor| {
            let old_parent = visitor.parent;
            let old_configuration = Rc::clone(&visitor.configuration);

            if loads_user_defined_modules {
                visitor.parent = Some(CssTree::ROOT);
            }

            // This configuration is only used if it passes through a `@forward`
            // rule, so we avoid creating unnecessary ones for performance reasons.
            if !stylesheet.forwards.is_empty() {
                visitor.configuration = Rc::new(RefCell::new(env.to_implicit_configuration()));
            }

            visitor.visit_stylesheet(stylesheet)?;

            if loads_user_defined_modules {
                visitor.parent = old_parent;
            }
            visitor.configuration = old_configuration;

            Ok(())
        })?;

        // Create a dummy module with empty CSS and no extensions to make forwarded
        // members available in the current import context and to combine all the
        // CSS from modules used by [stylesheet].
        let module = env.to_dummy_module(self.empty_span);
        self.env.import_forwards(module);

        if loads_user_defined_modules {
            // todo:
            //     if (module.transitivelyContainsCss) {
            //       // If any transitively used module contains extensions, we need to
            //       // clone all modules' CSS. Otherwise, it's possible that they'll be
            //       // used or imported from another location that shouldn't have the same
            //       // extensions applied.
            //       await _combineCss(module,
            //               clone: module.transitivelyContainsExtensions)
            //           .accept(this);
            //     }

            //     var visitor = _ImportedCssVisitor(this);
            //     for (var child in children) {
            //       child.accept(visitor);
            //     }
        }

        self.active_modules.remove(&url);

        Ok(())
    }

    pub(super) fn visit_static_import_rule(
        &mut self,
        static_import: AstPlainCssImport,
    ) -> SassResult<()> {
        let import = self.interpolation_to_value(static_import.url, false, false)?;

        let modifiers = static_import
            .modifiers
            .map(|modifiers| self.interpolation_to_value(modifiers, false, false))
            .transpose()?;

        let node = CssStmt::Import(import, modifiers);

        if self.parent.is_some() && self.parent != Some(CssTree::ROOT) {
            self.copy_parent_after_sibling();
            self.css_tree.add_stmt(node, self.parent);
        } else {
            self.import_nodes.push(node);
        }

        Ok(())
    }
}
