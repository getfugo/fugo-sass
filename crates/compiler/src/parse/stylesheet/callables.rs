//! Functions and mixins: `@function`, `@return`, `@mixin`, `@include`, `@content` and parameter lists.

use super::*;

/// Functions and mixins: `@function`, `@return`, `@mixin`, `@include`, `@content` and parameter lists.
pub(crate) trait CallableParser<'a>: StylesheetParser<'a> {
    fn parse_argument_declaration(&mut self) -> SassResult<ArgumentDeclaration> {
        self.expect_char('(')?;
        self.whitespace_with_newlines()?;

        let mut arguments = Vec::new();
        let mut named = HashSet::new();

        let mut rest_argument: Option<Identifier> = None;

        while self.toks_mut().next_char_is('$') {
            let name_start = self.toks().cursor();
            let name = Identifier::from(self.parse_variable_name()?);
            let name_span = self.toks_mut().span_from(name_start);
            self.whitespace_with_newlines()?;

            let mut default_value: Option<AstExpr> = None;

            if self.scan_char(':') {
                self.whitespace_with_newlines()?;
                default_value = Some(self.parse_expression_until_comma(false)?.node);
            } else if self.scan_char('.') {
                self.expect_char('.')?;
                self.expect_char('.')?;
                self.whitespace_with_newlines()?;
                if self.scan_char(',') {
                    self.whitespace_with_newlines()?;
                }
                rest_argument = Some(name);
                break;
            }

            arguments.push(Argument {
                name,
                default: default_value,
            });

            if !named.insert(name) {
                return Err(("Duplicate argument.", name_span).into());
            }

            if !self.scan_char(',') {
                break;
            }
            self.whitespace_with_newlines()?;
        }
        self.expect_char(')')?;

        Ok(ArgumentDeclaration {
            args: arguments,
            rest: rest_argument,
        })
    }

    fn parse_content_rule(&mut self, start: usize) -> SassResult<AstStmt> {
        if !self.flags().in_mixin() {
            return Err((
                "@content is only allowed within mixin declarations.",
                self.toks_mut().span_from(start),
            )
                .into());
        }

        self.whitespace()?;

        let args = if self.toks_mut().next_char_is('(') {
            self.parse_argument_invocation(true, false)?
        } else {
            ArgumentInvocation::empty(self.toks().current_span())
        };

        self.expect_statement_separator(Some("@content rule"))?;

        self.flags_mut().set(ContextFlags::FOUND_CONTENT_RULE, true);

        Ok(AstStmt::ContentRule(AstContentRule { args }))
    }

    fn parse_function_rule(&mut self, start: usize) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let name_start = self.toks().cursor();
        let name = self.parse_identifier(true, false)?;
        let name_span = self.toks_mut().span_from(name_start);
        self.whitespace_with_newlines()?;
        let arguments = self.parse_argument_declaration()?;

        if self.flags().in_mixin() || self.flags().in_content_block() {
            return Err((
                "Mixins may not contain function declarations.",
                self.toks_mut().span_from(start),
            )
                .into());
        } else if self.flags().in_control_flow() {
            return Err((
                "Functions may not be declared in control directives.",
                self.toks_mut().span_from(start),
            )
                .into());
        }

        if RESERVED_IDENTIFIERS.contains(&unvendor(&name)) {
            return Err(("Invalid function name.", self.toks_mut().span_from(start)).into());
        }

        self.whitespace()?;

        let children = self.with_children(Self::function_child)?.node;

        Ok(AstStmt::FunctionDecl(AstFunctionDecl {
            name: Spanned {
                node: Identifier::from(name),
                span: name_span,
            },
            arguments,
            body: children,
        }))
    }

    fn function_child(&mut self) -> SassResult<AstStmt> {
        let start = self.toks().cursor();
        if !self.toks_mut().next_char_is('@') {
            match self.parse_variable_declaration_with_namespace() {
                Ok(decl) => return Ok(AstStmt::VariableDecl(decl)),
                Err(e) => {
                    self.toks_mut().set_cursor(start);
                    let stmt = match self.parse_declaration_or_style_rule() {
                        Ok(stmt) => stmt,
                        Err(..) => return Err(e),
                    };

                    let (is_style_rule, span) = match stmt {
                        AstStmt::RuleSet(ruleset) => (true, ruleset.span),
                        AstStmt::Style(style) => (false, style.span),
                        _ => unreachable!(),
                    };

                    return Err((
                        format!(
                            "@function rules may not contain {}.",
                            if is_style_rule {
                                "style rules"
                            } else {
                                "declarations"
                            }
                        ),
                        span,
                    )
                        .into());
                }
            }
        }

        match self.plain_at_rule_name()?.as_str() {
            "debug" => self.parse_debug_rule(),
            "each" => self.parse_each_rule(Self::function_child),
            "else" => self.parse_disallowed_at_rule(start),
            "error" => self.parse_error_rule(),
            "for" => self.parse_for_rule(Self::function_child),
            "if" => self.parse_if_rule(Self::function_child),
            "return" => self.parse_return_rule(),
            "warn" => self.parse_warn_rule(),
            "while" => self.parse_while_rule(Self::function_child),
            _ => self.parse_disallowed_at_rule(start),
        }
    }

    fn parse_include_rule(&mut self) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let mut namespace: Option<Spanned<Identifier>> = None;

        let name_start = self.toks().cursor();
        let mut name = self.parse_identifier(false, false)?;

        if self.scan_char('.') {
            let namespace_span = self.toks_mut().span_from(name_start);
            namespace = Some(Spanned {
                node: Identifier::namespace(&name),
                span: namespace_span,
            });
            name = self.parse_public_identifier()?;
        } else {
            name = name.replace('_', "-");
        }

        let name = Identifier::from(name);
        let name_span = self.toks_mut().span_from(name_start);

        self.whitespace()?;

        let args = if self.toks_mut().next_char_is('(') {
            self.parse_argument_invocation(true, false)?
        } else {
            ArgumentInvocation::empty(self.toks().current_span())
        };

        self.whitespace()?;

        let content_args = if self.scan_identifier("using", false)? {
            self.whitespace_with_newlines()?;
            let args = self.parse_argument_declaration()?;
            self.whitespace()?;
            Some(args)
        } else {
            None
        };

        let mut content_block: Option<AstContentBlock> = None;

        if content_args.is_some() || self.looking_at_children()? {
            let content_args = content_args.unwrap_or_else(ArgumentDeclaration::empty);
            let was_in_content_block = self.flags().in_content_block();
            self.flags_mut().set(ContextFlags::IN_CONTENT_BLOCK, true);
            let body = self.with_children(Self::parse_statement)?.node;
            content_block = Some(AstContentBlock {
                args: content_args,
                body,
            });
            self.flags_mut()
                .set(ContextFlags::IN_CONTENT_BLOCK, was_in_content_block);
        } else {
            self.expect_statement_separator(None)?;
        }

        Ok(AstStmt::Include(AstInclude {
            namespace,
            name: Spanned {
                node: name,
                span: name_span,
            },
            args,
            content: content_block,
            span: name_span,
        }))
    }

    fn parse_return_rule(&mut self) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let value = self.parse_expression(None, None, None)?;
        self.expect_statement_separator(Some("@return rule"))?;
        Ok(AstStmt::Return(AstReturn {
            val: value.node,
            span: value.span,
        }))
    }

    fn parse_mixin_rule(&mut self, start: usize) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let name = Identifier::from(self.parse_identifier(true, false)?);
        self.whitespace()?;
        let args = if self.toks_mut().next_char_is('(') {
            self.parse_argument_declaration()?
        } else {
            ArgumentDeclaration::empty()
        };

        if self.flags().in_mixin() || self.flags().in_content_block() {
            return Err((
                "Mixins may not contain mixin declarations.",
                self.toks_mut().span_from(start),
            )
                .into());
        } else if self.flags().in_control_flow() {
            return Err((
                "Mixins may not be declared in control directives.",
                self.toks_mut().span_from(start),
            )
                .into());
        }

        self.whitespace()?;

        let old_found_content_rule = self.flags().found_content_rule();
        self.flags_mut()
            .set(ContextFlags::FOUND_CONTENT_RULE, false);
        self.flags_mut().set(ContextFlags::IN_MIXIN, true);

        let body = self.with_children(Self::parse_statement)?.node;

        let has_content = self.flags_mut().found_content_rule();

        self.flags_mut()
            .set(ContextFlags::FOUND_CONTENT_RULE, old_found_content_rule);
        self.flags_mut().set(ContextFlags::IN_MIXIN, false);

        Ok(AstStmt::Mixin(AstMixin {
            name,
            args,
            body,
            has_content,
        }))
    }
}

impl<'a, T: StylesheetParser<'a>> CallableParser<'a> for T {}
