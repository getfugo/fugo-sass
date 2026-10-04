//! Control flow, `@include`, comments, variables and messages.

use super::*;

impl<'a> Visitor<'a> {
    pub(super) fn visit_debug_rule(
        &mut self,
        debug_rule: AstDebugRule,
    ) -> SassResult<Option<Value>> {
        if self.options.quiet {
            return Ok(None);
        }

        let message = self.visit_expr(debug_rule.value)?;
        let message = message.inspect(debug_rule.span)?;

        let loc = self.map.look_up_span(debug_rule.span);
        self.options.logger.debug(loc, message.as_str());

        Ok(None)
    }

    pub(super) fn visit_content_rule(
        &mut self,
        content_rule: AstContentRule,
    ) -> SassResult<Option<Value>> {
        let span = content_rule.args.span;
        if let Some(content) = &self.env.content {
            #[allow(mutable_borrow_reservation_conflict)]
            self.run_user_defined_callable(
                MaybeEvaledArguments::Invocation(content_rule.args),
                Arc::clone(content),
                &content.env.clone(),
                span,
                |content, visitor| {
                    for stmt in content.content.body.clone() {
                        let result = visitor.visit_stmt(stmt)?;
                        debug_assert!(result.is_none());
                    }

                    Ok(())
                },
            )?;
        }

        Ok(None)
    }

    pub(super) fn visit_function_decl(&mut self, fn_decl: AstFunctionDecl) {
        let name = fn_decl.name.node;
        // todo: independency

        let func = SassFunction::UserDefined(UserDefinedFunction {
            function: Arc::new(fn_decl),
            name,
            env: self.env.new_closure(),
        });

        self.env.insert_fn(func);
    }

    pub(super) fn visit_error_rule(
        &mut self,
        error_rule: AstErrorRule,
    ) -> SassResult<Box<SassError>> {
        let value = self
            .visit_expr(error_rule.value)?
            .inspect(error_rule.span)?;

        Ok((value, error_rule.span).into())
    }

    pub(super) fn visit_warn_rule(&mut self, warn_rule: AstWarn) -> SassResult<()> {
        if self.warnings_emitted.insert(warn_rule.span) {
            let value = self.visit_expr(warn_rule.value)?;
            let message = value.to_css_string(warn_rule.span, self.options.is_compressed())?;
            self.emit_warning(&message, warn_rule.span);
        }

        Ok(())
    }

    pub(super) fn visit_include_stmt(
        &mut self,
        include_stmt: AstInclude,
    ) -> SassResult<Option<Value>> {
        let mixin = self
            .env
            .get_mixin(include_stmt.name, include_stmt.namespace)?;

        let AstInclude {
            args,
            content,
            name,
            span,
            ..
        } = include_stmt;

        let content = content.map(|content| {
            Arc::new(CallableContentBlock {
                content,
                env: self.env.new_closure(),
            })
        });

        self.apply_mixin(
            mixin,
            content,
            MaybeEvaledArguments::Invocation(args),
            name.span,
            span,
        )?;

        Ok(None)
    }

    /// Runs `mixin` with `arguments` and `content`, for `@include` and `meta.apply()` (dart-sass's
    /// `_applyMixin`). `span` is the invocation's, `include_span` the whole rule's.
    pub(crate) fn apply_mixin(
        &mut self,
        mixin: Mixin,
        content: Option<Arc<CallableContentBlock>>,
        arguments: MaybeEvaledArguments,
        span: Span,
        include_span: Span,
    ) -> SassResult<()> {
        if content.is_some() && !mixin.accepts_content() {
            return Err(("Mixin doesn't accept a content block.", include_span).into());
        }

        let old_in_mixin = self.flags.in_mixin();
        self.flags.set(ContextFlags::IN_MIXIN, true);

        let result = match mixin {
            Mixin::Builtin { mixin, .. } => {
                let args = self.eval_maybe_args(arguments, span)?;
                self.with_content(content, |visitor| mixin(args, visitor))
            }
            Mixin::UserDefined(mixin, env) => self.run_user_defined_callable::<_, (), _>(
                arguments,
                mixin,
                &env,
                span,
                |mixin, visitor| {
                    visitor.with_content(content, |visitor| {
                        for stmt in mixin.body.clone() {
                            let result = visitor.visit_stmt(stmt)?;
                            debug_assert!(result.is_none());
                        }
                        Ok(())
                    })
                },
            ),
        };

        self.flags.set(ContextFlags::IN_MIXIN, old_in_mixin);

        result
    }

    pub(super) fn visit_mixin_decl(&mut self, mixin: AstMixin) {
        self.env.insert_mixin(
            mixin.name,
            Mixin::UserDefined(Arc::new(mixin), self.env.new_closure()),
        );
    }

    pub(super) fn visit_each_stmt(&mut self, each_stmt: AstEach) -> SassResult<Option<Value>> {
        let list = self.visit_expr(each_stmt.list)?.as_list();

        // todo: not setting semi_global: true maybe means we can't assign to global scope when declared as global
        self.env.scopes_mut().enter_new_scope();

        let mut result = None;

        'outer: for val in list {
            if each_stmt.variables.len() == 1 {
                let val = self.without_slash(val);
                self.env
                    .scopes_mut()
                    .insert_var_last(each_stmt.variables[0], val);
            } else {
                for (&var, val) in each_stmt.variables.iter().zip(
                    val.as_list()
                        .into_iter()
                        .chain(std::iter::once(Value::Null).cycle()),
                ) {
                    let val = self.without_slash(val);
                    self.env.scopes_mut().insert_var_last(var, val);
                }
            }

            for stmt in each_stmt.body.clone() {
                let val = self.visit_stmt(stmt)?;
                if val.is_some() {
                    result = val;
                    break 'outer;
                }
            }
        }

        self.env.scopes_mut().exit_scope();

        Ok(result)
    }

    pub(super) fn visit_for_stmt(&mut self, for_stmt: AstFor) -> SassResult<Option<Value>> {
        let from_span = for_stmt.from.span;
        let to_span = for_stmt.to.span;
        let from_number = self
            .visit_expr(for_stmt.from.node)?
            .assert_number(from_span)?;
        let to_number = self.visit_expr(for_stmt.to.node)?.assert_number(to_span)?;

        if !to_number.unit().comparable(from_number.unit()) {
            // todo: better error message here
            return Err((
                "to and from values have incompatible units",
                from_span.merge(to_span),
            )
                .into());
        }

        let from = from_number.num.assert_int(from_span)?;
        let mut to = to_number
            .num
            .convert(to_number.unit(), from_number.unit())
            .assert_int(to_span)?;

        let direction = if from > to { -1 } else { 1 };

        if to == i64::MAX || to == i64::MIN {
            return Err((
                "@for loop upper bound exceeds valid integer representation (i64::MAX)",
                to_span,
            )
                .into());
        }

        if !for_stmt.is_exclusive {
            to += direction;
        }

        if from == to {
            return Ok(None);
        }

        // todo: self.with_scopes
        self.env.scopes_mut().enter_new_scope();

        let mut result = None;

        let mut i = from;
        'outer: while i != to {
            self.env.scopes_mut().insert_var_last(
                for_stmt.variable.node,
                Value::Dimension(SassNumber {
                    num: Number::from(i),
                    unit: from_number.unit().clone(),
                    as_slash: None,
                }),
            );

            for stmt in for_stmt.body.clone() {
                let val = self.visit_stmt(stmt)?;
                if val.is_some() {
                    result = val;
                    break 'outer;
                }
            }

            i += direction;
        }

        self.env.scopes_mut().exit_scope();

        Ok(result)
    }

    pub(super) fn visit_while_stmt(&mut self, while_stmt: &AstWhile) -> SassResult<Option<Value>> {
        self.with_scope(true, true, |visitor| {
            let mut result = None;

            'outer: while visitor
                .visit_expr(while_stmt.condition.clone())?
                .is_truthy()
            {
                for stmt in while_stmt.body.clone() {
                    let val = visitor.visit_stmt(stmt)?;
                    if val.is_some() {
                        result = val;
                        break 'outer;
                    }
                }
            }

            Ok(result)
        })
    }

    pub(super) fn visit_if_stmt(&mut self, if_stmt: AstIf) -> SassResult<Option<Value>> {
        let mut clause: Option<Vec<AstStmt>> = if_stmt.else_clause;
        for clause_to_check in if_stmt.if_clauses {
            if self.visit_expr(clause_to_check.condition)?.is_truthy() {
                clause = Some(clause_to_check.body);
                break;
            }
        }

        // todo: self.with_scope
        self.env.scopes_mut().enter_new_scope();

        let mut result = None;

        if let Some(stmts) = clause {
            for stmt in stmts {
                let val = self.visit_stmt(stmt)?;
                if val.is_some() {
                    result = val;
                    break;
                }
            }
        }

        self.env.scopes_mut().exit_scope();

        Ok(result)
    }

    pub(super) fn visit_loud_comment(
        &mut self,
        comment: AstLoudComment,
    ) -> SassResult<Option<Value>> {
        if self.flags.in_function() {
            return Ok(None);
        }

        // todo: Comments are allowed to appear between CSS imports
        // if (_parent == _root && _endOfImports == _root.children.length) {
        //   _endOfImports++;
        // }

        let comment = CssStmt::Comment(
            self.perform_interpolation(comment.text, false)?,
            comment.span,
        );
        self.copy_parent_after_sibling();
        self.css_tree.add_stmt(comment, self.parent);

        Ok(None)
    }

    pub(super) fn visit_variable_decl(
        &mut self,
        decl: AstVariableDecl,
    ) -> SassResult<Option<Value>> {
        let name = Spanned {
            node: decl.name,
            span: decl.span,
        };

        if decl.is_guarded {
            if decl.namespace.is_none() && self.env.at_root() {
                let var_override = (*self.configuration).borrow_mut().remove(decl.name);
                if !matches!(
                    var_override,
                    Some(ConfiguredValue {
                        value: Value::Null,
                        ..
                    }) | None
                ) {
                    self.env.insert_var(
                        name,
                        None,
                        var_override.unwrap().value,
                        true,
                        self.flags.in_semi_global_scope(),
                    )?;
                    return Ok(None);
                }
            }

            if self.env.var_exists(decl.name, decl.namespace)? {
                let value = self.env.get_var(name, decl.namespace).unwrap();

                if value != Value::Null {
                    return Ok(None);
                }
            }
        }

        let value = self.visit_expr(decl.value)?;
        let value = self.without_slash(value);

        self.env.insert_var(
            name,
            decl.namespace,
            value,
            decl.is_global,
            self.flags.in_semi_global_scope(),
        )?;

        Ok(None)
    }
}
