//! Running functions, mixins and content blocks with their arguments.

use super::*;

pub(crate) trait UserDefinedCallable {
    fn name(&self) -> Identifier;
    fn arguments(&self) -> &ArgumentDeclaration;
}

#[derive(Debug, Clone)]
pub(crate) struct CallableContentBlock {
    pub(super) content: AstContentBlock,
    pub(super) env: Environment,
}

impl<'a> Visitor<'a> {
    pub(super) fn eval_maybe_args(
        &mut self,
        args: MaybeEvaledArguments,
        span: Span,
    ) -> SassResult<ArgumentResult> {
        match args {
            MaybeEvaledArguments::Invocation(args) => self.eval_args(args, span),
            MaybeEvaledArguments::Evaled(args) => Ok(args),
        }
    }

    pub(super) fn eval_args(
        &mut self,
        arguments: ArgumentInvocation,
        span: Span,
    ) -> SassResult<ArgumentResult> {
        let mut positional = Vec::with_capacity(arguments.positional.len());

        for expr in arguments.positional {
            let val = self.visit_expr(expr)?;
            positional.push(self.without_slash(val));
        }

        let mut named = IndexMap::new();

        for (key, expr) in arguments.named {
            let val = self.visit_expr(expr)?;
            named.insert(key, self.without_slash(val));
        }

        if arguments.rest.is_none() {
            return Ok(ArgumentResult {
                positional,
                named,
                separator: ListSeparator::Undecided,
                span,
                touched: BTreeSet::new(),
            });
        }

        let rest = self.visit_expr(arguments.rest.unwrap())?;

        let mut separator = ListSeparator::Undecided;

        match rest {
            Value::Map(rest) => self.add_rest_map(&mut named, rest)?,
            Value::List(elems, list_separator, _) => {
                let mut list = elems
                    .into_iter()
                    .map(|e| self.without_slash(e))
                    .collect::<Vec<_>>();
                positional.append(&mut list);
                separator = list_separator;
            }
            Value::ArgList(arglist) => {
                // todo: superfluous clone
                for (&key, value) in arglist.keywords() {
                    named.insert(key, self.without_slash(value.clone()));
                }

                let mut list = arglist
                    .elems
                    .into_iter()
                    .map(|e| self.without_slash(e))
                    .collect::<Vec<_>>();
                positional.append(&mut list);
                separator = arglist.separator;
            }
            _ => {
                positional.push(self.without_slash(rest));
            }
        }

        if arguments.keyword_rest.is_none() {
            return Ok(ArgumentResult {
                positional,
                named,
                separator: ListSeparator::Undecided,
                span: arguments.span,
                touched: BTreeSet::new(),
            });
        }

        match self.visit_expr(arguments.keyword_rest.unwrap())? {
            Value::Map(keyword_rest) => {
                self.add_rest_map(&mut named, keyword_rest)?;

                Ok(ArgumentResult {
                    positional,
                    named,
                    separator,
                    span: arguments.span,
                    touched: BTreeSet::new(),
                })
            }
            v => Err((
                format!(
                    "Variable keyword arguments must be a map (was {}).",
                    v.inspect(arguments.span)?
                ),
                arguments.span,
            )
                .into()),
        }
    }

    pub(super) fn add_rest_map(
        &mut self,
        named: &mut IndexMap<Identifier, Value>,
        rest: SassMap,
    ) -> SassResult<()> {
        for (key, val) in rest {
            match key.node {
                Value::String(text, ..) => {
                    let val = self.without_slash(val);
                    named.insert(Identifier::from(text), val);
                }
                _ => {
                    return Err((
                        // todo: we have to render the map for this error message
                        "Variable keyword argument map must have string keys.",
                        key.span,
                    )
                        .into());
                }
            }
        }

        Ok(())
    }

    pub(super) fn run_user_defined_callable<
        F: UserDefinedCallable,
        V: fmt::Debug,
        R: FnOnce(F, &mut Self) -> SassResult<V>,
    >(
        &mut self,
        arguments: MaybeEvaledArguments,
        func: F,
        env: &Environment,
        span: Span,
        run: R,
    ) -> SassResult<V> {
        let mut evaluated = self.eval_maybe_args(arguments, span)?;

        let mut name = func.name().to_string();

        if name != "@content" {
            name.push_str("()");
        }

        self.with_environment(env.new_closure(), |visitor| {
            visitor.with_scope(false, true, move |visitor| {
                func.arguments().verify(
                    evaluated.positional.len(),
                    &evaluated.named,
                    evaluated.span,
                )?;

                let declared_arguments = &func.arguments().args;
                let min_len = evaluated.positional.len().min(declared_arguments.len());

                let positional_len = evaluated.positional.len();

                #[allow(clippy::needless_range_loop)]
                for i in (0..min_len).rev() {
                    visitor.env.scopes_mut().insert_var_last(
                        declared_arguments[i].name,
                        evaluated.positional.remove(i),
                    );
                }

                // todo: better name for var
                let additional_declared_args = if declared_arguments.len() > positional_len {
                    &declared_arguments[positional_len..declared_arguments.len()]
                } else {
                    &[]
                };

                for argument in additional_declared_args {
                    let name = argument.name;
                    let value = evaluated.named.shift_remove(&argument.name).map_or_else(
                        || {
                            // todo: superfluous clone
                            let v = visitor.visit_expr(argument.default.clone().unwrap())?;
                            Ok(visitor.without_slash(v))
                        },
                        SassResult::Ok,
                    )?;
                    visitor.env.scopes_mut().insert_var_last(name, value);
                }

                let were_keywords_accessed = Rc::new(Cell::new(false));

                let num_named_args = evaluated.named.len();

                let has_arg_list = if let Some(rest_arg) = func.arguments().rest {
                    let rest = if !evaluated.positional.is_empty() {
                        evaluated.positional
                    } else {
                        Vec::new()
                    };

                    let arg_list = Value::ArgList(ArgList::new(
                        rest,
                        Rc::clone(&were_keywords_accessed),
                        // todo: superfluous clone
                        evaluated.named.clone(),
                        if evaluated.separator == ListSeparator::Undecided {
                            ListSeparator::Comma
                        } else {
                            ListSeparator::Space
                        },
                    ));

                    visitor.env.scopes_mut().insert_var_last(rest_arg, arg_list);

                    true
                } else {
                    false
                };

                let val = run(func, visitor)?;

                if !has_arg_list || num_named_args == 0 {
                    return Ok(val);
                }

                if (*were_keywords_accessed).get() {
                    return Ok(val);
                }

                let argument_word = if num_named_args == 1 {
                    "argument"
                } else {
                    "arguments"
                };

                let argument_names = to_sentence(
                    evaluated
                        .named
                        .keys()
                        .map(|key| format!("${key}", key = key))
                        .collect(),
                    "or",
                );

                Err((
                    format!(
                        "No {argument_word} named {argument_names}.",
                        argument_word = argument_word,
                        argument_names = argument_names
                    ),
                    span,
                )
                    .into())
            })
        })
    }

    pub(crate) fn run_function_callable(
        &mut self,
        func: SassFunction,
        arguments: ArgumentInvocation,
        span: Span,
    ) -> SassResult<Value> {
        self.run_function_callable_with_maybe_evaled(
            func,
            MaybeEvaledArguments::Invocation(arguments),
            span,
        )
    }

    pub(crate) fn run_function_callable_with_maybe_evaled(
        &mut self,
        func: SassFunction,
        arguments: MaybeEvaledArguments,
        span: Span,
    ) -> SassResult<Value> {
        match func {
            SassFunction::Builtin(func, _name) => {
                let evaluated = self.eval_maybe_args(arguments, span)?;
                let val = func.0(evaluated, self)?;
                Ok(self.without_slash(val))
            }
            SassFunction::UserDefined(UserDefinedFunction { function, env, .. }) => self
                .run_user_defined_callable(arguments, function, &env, span, |function, visitor| {
                    for stmt in function.body.clone() {
                        let result = visitor.visit_stmt(stmt)?;

                        if let Some(val) = result {
                            return Ok(val);
                        }
                    }

                    Err(("Function finished without @return.", span).into())
                }),
            SassFunction::Plain { name } => {
                let has_named;
                let mut rest = None;

                // todo: somewhat hacky solution to support plain css fns passed
                // as strings to `call(..)`
                let arguments = match arguments {
                    MaybeEvaledArguments::Invocation(args) => {
                        has_named = !args.named.is_empty() || args.keyword_rest.is_some();
                        rest = args.rest;
                        args.positional
                            .into_iter()
                            .map(|arg| self.evaluate_to_css(arg, QuoteKind::Quoted, span))
                            .collect::<SassResult<Vec<_>>>()?
                    }
                    MaybeEvaledArguments::Evaled(args) => {
                        has_named = !args.named.is_empty();

                        args.positional
                            .into_iter()
                            .map(|arg| arg.to_css_string(span, self.options.is_compressed()))
                            .collect::<SassResult<Vec<_>>>()?
                    }
                };

                if has_named {
                    return Err(
                        ("Plain CSS functions don't support keyword arguments.", span).into(),
                    );
                }

                let mut buffer = format!("{}(", name.as_str());
                let mut first = true;

                for argument in arguments {
                    if first {
                        first = false;
                    } else {
                        buffer.push_str(", ");
                    }

                    buffer.push_str(&argument);
                }

                if let Some(rest_arg) = rest {
                    let rest = self.visit_expr(rest_arg)?;
                    if !first {
                        buffer.push_str(", ");
                    }
                    buffer.push_str(&self.serialize(rest, QuoteKind::Quoted, span)?);
                }
                buffer.push(')');

                Ok(Value::String(buffer, QuoteKind::None))
            }
        }
    }
}

impl UserDefinedCallable for AstFunctionDecl {
    fn name(&self) -> Identifier {
        self.name.node
    }

    fn arguments(&self) -> &ArgumentDeclaration {
        &self.arguments
    }
}

impl UserDefinedCallable for Arc<AstFunctionDecl> {
    fn name(&self) -> Identifier {
        self.name.node
    }

    fn arguments(&self) -> &ArgumentDeclaration {
        &self.arguments
    }
}

impl UserDefinedCallable for AstMixin {
    fn name(&self) -> Identifier {
        self.name
    }

    fn arguments(&self) -> &ArgumentDeclaration {
        &self.args
    }
}

impl UserDefinedCallable for Arc<AstMixin> {
    fn name(&self) -> Identifier {
        self.name
    }

    fn arguments(&self) -> &ArgumentDeclaration {
        &self.args
    }
}

impl UserDefinedCallable for Arc<CallableContentBlock> {
    fn name(&self) -> Identifier {
        Identifier::from("@content")
    }

    fn arguments(&self) -> &ArgumentDeclaration {
        &self.content.args
    }
}
