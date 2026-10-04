//! Modules: `@use` and `@forward`.

use super::*;

/// Modules: `@use` and `@forward`.
pub(crate) trait ModuleParser<'a>: StylesheetParser<'a> {
    fn parse_public_identifier(&mut self) -> SassResult<String> {
        let start = self.toks().cursor();
        let ident = self.parse_identifier(true, false)?;
        Self::assert_public(&ident, self.toks_mut().span_from(start))?;

        Ok(ident)
    }

    fn parse_forward_rule(&mut self, start: usize) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let url = PathBuf::from(self.parse_url_string()?);
        self.whitespace()?;

        let prefix = if self.scan_identifier("as", false)? {
            self.whitespace_with_newlines()?;
            let prefix = self.parse_identifier(true, false)?;
            self.expect_char('*')?;
            self.whitespace()?;
            Some(prefix)
        } else {
            None
        };

        let mut shown_mixins_and_functions: Option<IndexSet<Identifier>> = None;
        let mut shown_variables: Option<IndexSet<Identifier>> = None;
        let mut hidden_mixins_and_functions: Option<IndexSet<Identifier>> = None;
        let mut hidden_variables: Option<IndexSet<Identifier>> = None;

        if self.scan_identifier("show", false)? {
            let members = self.parse_member_list()?;
            shown_mixins_and_functions = Some(members.0);
            shown_variables = Some(members.1);
        } else if self.scan_identifier("hide", false)? {
            let members = self.parse_member_list()?;
            hidden_mixins_and_functions = Some(members.0);
            hidden_variables = Some(members.1);
        }

        let config = self.parse_configuration(true)?;

        self.expect_statement_separator(Some("@forward rule"))?;
        let span = self.toks_mut().span_from(start);

        if !self.flags().is_use_allowed() {
            return Err((
                "@forward rules must be written before any other rules.",
                span,
            )
                .into());
        }

        Ok(AstStmt::Forward(
            if let (Some(shown_mixins_and_functions), Some(shown_variables)) =
                (shown_mixins_and_functions, shown_variables)
            {
                AstForwardRule::show(
                    url,
                    shown_mixins_and_functions,
                    shown_variables,
                    prefix,
                    config,
                    span,
                )
            } else if let (Some(hidden_mixins_and_functions), Some(hidden_variables)) =
                (hidden_mixins_and_functions, hidden_variables)
            {
                AstForwardRule::hide(
                    url,
                    hidden_mixins_and_functions,
                    hidden_variables,
                    prefix,
                    config,
                    span,
                )
            } else {
                AstForwardRule::new(url, prefix, config, span)
            },
        ))
    }

    fn parse_member_list(&mut self) -> SassResult<(IndexSet<Identifier>, IndexSet<Identifier>)> {
        let mut identifiers = IndexSet::new();
        let mut variables = IndexSet::new();

        loop {
            self.whitespace_with_newlines()?;

            // todo: withErrorMessage("Expected variable, mixin, or function name"
            if self.toks_mut().next_char_is('$') {
                variables.insert(Identifier::from(self.parse_variable_name()?));
            } else {
                identifiers.insert(Identifier::from(self.parse_identifier(true, false)?));
            }

            self.whitespace()?;

            if !self.scan_char(',') {
                break;
            }
        }

        Ok((identifiers, variables))
    }

    fn parse_url_string(&mut self) -> SassResult<String> {
        // todo: real uri parsing
        self.parse_string()
    }

    fn use_namespace(
        &mut self,
        url: &Path,
        _start: usize,
        url_span: Span,
    ) -> SassResult<Option<String>> {
        if self.scan_identifier("as", false)? {
            self.whitespace_with_newlines()?;
            return Ok(if self.scan_char('*') {
                None
            } else {
                Some(self.parse_identifier(false, false)?)
            });
        }

        let base_name = url
            .file_name()
            .map_or_else(OsString::new, ToOwned::to_owned);
        let base_name = base_name.to_string_lossy();
        let dot = base_name.find('.');

        let start = if base_name.starts_with('_') { 1 } else { 0 };
        let end = dot.unwrap_or(base_name.len());
        let namespace = if url.to_string_lossy().starts_with("sass:") {
            return Ok(Some(url.to_string_lossy().into_owned()));
        } else {
            &base_name[start..end]
        };

        let mut toks = Lexer::new_from_string(namespace, url_span);

        // if namespace is empty, avoid attempting to parse an identifier from
        // an empty string, as there will be no span to emit
        let identifier = if namespace.is_empty() {
            Err(("", self.empty_span()).into())
        } else {
            mem::swap(self.toks_mut(), &mut toks);
            let ident = self.parse_identifier(false, false);
            mem::swap(self.toks_mut(), &mut toks);
            ident
        };

        match (identifier, toks.peek().is_none()) {
            (Ok(i), true) => Ok(Some(i)),
            _ => {
                Err((
                    format!(
                        "The default namespace \"{namespace}\" is not a valid Sass identifier.\n\nRecommendation: add an \"as\" clause to define an explicit namespace.", 
                        namespace = namespace
                    ),
                    self.toks_mut().span_from(start)
                ).into())
            }
        }
    }

    fn parse_configuration(
        &mut self,
        // default=false
        allow_guarded: bool,
    ) -> SassResult<Option<Vec<ConfiguredVariable>>> {
        if !self.scan_identifier("with", false)? {
            return Ok(None);
        }

        let mut variable_names = HashSet::new();
        let mut configuration = Vec::new();
        self.whitespace_with_newlines()?;
        self.expect_char('(')?;

        loop {
            self.whitespace_with_newlines()?;
            let var_start = self.toks().cursor();
            let name = Identifier::from(self.parse_variable_name()?);
            let name_span = self.toks_mut().span_from(var_start);
            self.whitespace_with_newlines()?;
            self.expect_char(':')?;
            self.whitespace_with_newlines()?;
            let expr = self.parse_expression_until_comma(false)?;

            let mut is_guarded = false;
            let flag_start = self.toks().cursor();
            if allow_guarded && self.scan_char('!') {
                let flag = self.parse_identifier(false, false)?;
                if flag == "default" {
                    is_guarded = true;
                    self.whitespace_with_newlines()?;
                } else {
                    return Err(
                        ("Invalid flag name.", self.toks_mut().span_from(flag_start)).into(),
                    );
                }
            }

            let span = self.toks_mut().span_from(var_start);
            if variable_names.contains(&name) {
                return Err(("The same variable may only be configured once.", span).into());
            }

            variable_names.insert(name);
            configuration.push(ConfiguredVariable {
                name: Spanned {
                    node: name,
                    span: name_span,
                },
                expr,
                is_guarded,
            });

            if !self.scan_char(',') {
                break;
            }
            self.whitespace_with_newlines()?;
            if !self.looking_at_expression() {
                break;
            }
        }

        self.expect_char(')')?;

        Ok(Some(configuration))
    }

    fn parse_use_rule(&mut self, start: usize) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let url_start = self.toks().cursor();
        let url = self.parse_url_string()?;
        let url_span = self.toks().span_from(url_start);
        self.whitespace()?;

        let path = PathBuf::from(url);

        let namespace = self.use_namespace(path.as_ref(), start, url_span)?;
        self.whitespace()?;
        let configuration = self.parse_configuration(false)?;

        self.expect_statement_separator(Some("@use rule"))?;

        let span = self.toks_mut().span_from(start);

        if !self.flags().is_use_allowed() {
            return Err((
                "@use rules must be written before any other rules.",
                self.toks_mut().span_from(start),
            )
                .into());
        }

        self.expect_statement_separator(Some("@use rule"))?;

        Ok(AstStmt::Use(AstUseRule {
            url: path,
            namespace,
            configuration: configuration.unwrap_or_default(),
            span,
        }))
    }

    fn assert_public(ident: &str, span: Span) -> SassResult<()> {
        if !ScssParser::is_private(ident) {
            return Ok(());
        }

        Err((
            "Private members can't be accessed from outside their modules.",
            span,
        )
            .into())
    }

    fn is_private(ident: &str) -> bool {
        ident.starts_with('-') || ident.starts_with('_')
    }
}

impl<'a, T: StylesheetParser<'a>> ModuleParser<'a> for T {}
