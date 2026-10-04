//! Expressions.

use super::*;

impl<'a> Visitor<'a> {
    #[allow(clippy::unused_self)]
    pub(super) fn without_slash(&mut self, v: Value) -> Value {
        match v {
            Value::Dimension(SassNumber { .. }) if v.as_slash().is_some() => {
                // todo: emit warning. we don't currently because it can be quite loud
                // self.emit_warning(
                //     Cow::Borrowed("Using / for division is deprecated and will be removed at some point in the future"),
                //     self.empty_span,
                // );
            }
            _ => {}
        }

        v.without_slash()
    }

    pub(super) fn visit_list_expr(&mut self, list: ListExpr) -> SassResult<Value> {
        let elems = list
            .elems
            .into_iter()
            .map(|e| {
                let value = self.visit_expr(e.node)?;
                Ok(value)
            })
            .collect::<SassResult<Vec<_>>>()?;

        Ok(Value::List(elems, list.separator, list.brackets))
    }

    pub(super) fn visit_function_call_expr(
        &mut self,
        func_call: FunctionCallExpr,
    ) -> SassResult<Value> {
        let name = func_call.name;

        // As dart-sass's `visitFunctionExpression`: a Sass function of the name wins; then the
        // calculations; then the built-in and plain CSS functions.
        let func = if self.is_plain_css {
            None
        } else {
            self.env.get_fn(name, func_call.namespace)?
        };
        let func = match func {
            Some(func) => func,
            None => {
                if func_call.namespace.is_some() {
                    return Err(("Undefined function.", func_call.span).into());
                }

                let lower = name.as_str().to_ascii_lowercase();
                match lower.as_str() {
                    "min" | "max" | "round" | "abs"
                        if func_call.arguments.named.is_empty()
                            && func_call.arguments.rest.is_none()
                            && func_call
                                .arguments
                                .positional
                                .iter()
                                .all(AstExpr::is_calculation_safe) =>
                    {
                        return self.visit_calculation(&func_call, Some(&lower));
                    }
                    "calc" | "clamp" | "hypot" | "sin" | "cos" | "tan" | "asin" | "acos"
                    | "atan" | "sqrt" | "exp" | "sign" | "mod" | "rem" | "atan2" | "pow"
                    | "log" | "calc-size" => {
                        return self.visit_calculation(&func_call, None);
                    }
                    _ => {}
                }

                if self.is_plain_css {
                    SassFunction::Plain { name }
                } else if let Some(f) = self.options.custom_fns.get(name.as_str()) {
                    SassFunction::Builtin(f.clone(), name)
                } else if let Some(f) = GLOBAL_FUNCTIONS.get(name.as_str()) {
                    SassFunction::Builtin(f.clone(), name)
                } else {
                    SassFunction::Plain { name }
                }
            }
        };

        let old_in_function = self.flags.in_function();
        self.flags.set(ContextFlags::IN_FUNCTION, true);
        let value =
            self.run_function_callable(func, (*func_call.arguments).clone(), func_call.span)?;
        self.flags.set(ContextFlags::IN_FUNCTION, old_in_function);

        Ok(value)
    }

    pub(super) fn visit_interpolated_func_expr(
        &mut self,
        func: InterpolatedFunction,
    ) -> SassResult<Value> {
        let InterpolatedFunction {
            name,
            arguments: args,
            span,
        } = func;
        let fn_name = self.perform_interpolation(name, false)?;

        if !args.named.is_empty() || args.keyword_rest.is_some() {
            return Err(("Plain CSS functions don't support keyword arguments.", span).into());
        }

        let mut buffer = format!("{}(", fn_name);

        let mut first = true;
        for arg in args.positional.clone() {
            if first {
                first = false;
            } else {
                buffer.push_str(", ");
            }
            let evaluated = self.evaluate_to_css(arg, QuoteKind::Quoted, span)?;
            buffer.push_str(&evaluated);
        }

        if let Some(rest_arg) = args.rest {
            let rest = self.visit_expr(rest_arg)?;
            if !first {
                buffer.push_str(", ");
            }
            buffer.push_str(&self.serialize(rest, QuoteKind::None, span)?);
        }

        buffer.push(')');

        Ok(Value::String(buffer, QuoteKind::None))
    }

    pub(super) fn visit_parent_selector(&self) -> Value {
        match &self.style_rule_ignoring_at_root {
            Some(selector) => selector.as_selector_list().clone().to_sass_list(),
            None => Value::Null,
        }
    }

    pub(super) fn visit_expr(&mut self, expr: AstExpr) -> SassResult<Value> {
        Ok(match expr {
            AstExpr::Color(color) => Value::Color(color),
            AstExpr::Number { n, unit } => Value::Dimension(SassNumber {
                num: n,
                unit,
                as_slash: None,
            }),
            AstExpr::List(list) => self.visit_list_expr(list)?,
            AstExpr::String(StringExpr(text, quote), ..) => self.visit_string(text, quote)?,
            AstExpr::BinaryOp(binop) => self.visit_bin_op(
                binop.lhs.clone(),
                binop.op,
                binop.rhs.clone(),
                binop.allows_slash,
                binop.span,
            )?,
            AstExpr::True => Value::True,
            AstExpr::False => Value::False,
            AstExpr::FunctionCall(func_call) => self.visit_function_call_expr(func_call)?,
            AstExpr::If(if_expr) => self.visit_if_expr(&if_expr)?,
            AstExpr::LegacyIf(if_expr) => self.visit_ternary((*if_expr).clone())?,
            AstExpr::InterpolatedFunction(func) => {
                self.visit_interpolated_func_expr((*func).clone())?
            }
            AstExpr::Map(map) => self.visit_map(map)?,
            AstExpr::Null => Value::Null,
            AstExpr::Paren(expr) => {
                if self.is_plain_css {
                    return Err(
                        ("Parentheses aren't allowed in plain CSS.", self.empty_span).into(),
                    );
                }
                self.visit_expr((*expr).clone())?
            }
            AstExpr::ParentSelector => self.visit_parent_selector(),
            AstExpr::UnaryOp(op, expr, span) => self.visit_unary_op(op, (*expr).clone(), span)?,
            AstExpr::Variable { name, namespace } => self.env.get_var(name, namespace)?,
            AstExpr::Supports(condition) => Value::String(
                self.visit_supports_condition((*condition).clone())?,
                QuoteKind::None,
            ),
        })
    }

    pub(super) fn visit_unary_op(
        &mut self,
        op: UnaryOp,
        expr: AstExpr,
        span: Span,
    ) -> SassResult<Value> {
        let operand = self.visit_expr(expr)?;

        match op {
            UnaryOp::Plus => operand.unary_plus(self, span),
            UnaryOp::Neg => operand.unary_neg(self, span),
            UnaryOp::Div => operand.unary_div(self, span),
            UnaryOp::Not => Ok(operand.unary_not()),
        }
    }

    pub(super) fn visit_ternary(&mut self, if_expr: Ternary) -> SassResult<Value> {
        if_arguments().verify(if_expr.0.positional.len(), &if_expr.0.named, if_expr.0.span)?;

        let mut positional = if_expr.0.positional;
        let mut named = if_expr.0.named;

        let condition = if positional.is_empty() {
            named.shift_remove(&Identifier::from("condition")).unwrap()
        } else {
            positional.remove(0)
        };

        let if_true = if positional.is_empty() {
            named.shift_remove(&Identifier::from("if_true")).unwrap()
        } else {
            positional.remove(0)
        };

        let if_false = if positional.is_empty() {
            named.shift_remove(&Identifier::from("if_false")).unwrap()
        } else {
            positional.remove(0)
        };

        let value = if self.visit_expr(condition)?.is_truthy() {
            self.visit_expr(if_true)?
        } else {
            self.visit_expr(if_false)?
        };

        Ok(self.without_slash(value))
    }

    pub(super) fn visit_string(
        &mut self,
        mut text: Interpolation,
        quote: QuoteKind,
    ) -> SassResult<Value> {
        // Don't use [performInterpolation] here because we need to get the raw text
        // from strings, rather than the semantic value.
        let old_in_supports_declaration = self.flags.in_supports_declaration();
        self.flags.set(ContextFlags::IN_SUPPORTS_DECLARATION, false);

        let result = match text.contents.len() {
            0 => String::new(),
            1 => match text.contents.pop() {
                Some(InterpolationPart::String(s)) => s,
                Some(InterpolationPart::Expr(Spanned { node, span })) => {
                    match self.visit_expr(node)? {
                        Value::String(s, ..) => s,
                        e => self.serialize(e, QuoteKind::None, span)?,
                    }
                }
                None => unreachable!(),
            },
            _ => text
                .contents
                .into_iter()
                .map(|part| match part {
                    InterpolationPart::String(s) => Ok(s),
                    InterpolationPart::Expr(Spanned { node, span }) => {
                        match self.visit_expr(node)? {
                            Value::String(s, ..) => Ok(s),
                            e => self.serialize(e, QuoteKind::None, span),
                        }
                    }
                })
                .collect::<SassResult<String>>()?,
        };

        self.flags.set(
            ContextFlags::IN_SUPPORTS_DECLARATION,
            old_in_supports_declaration,
        );

        Ok(Value::String(result, quote))
    }

    pub(super) fn visit_map(&mut self, map: AstSassMap) -> SassResult<Value> {
        let mut sass_map = SassMap::new();

        for pair in map.0 {
            let key_span = pair.0.span;
            let key = self.visit_expr(pair.0.node)?;
            let value = self.visit_expr(pair.1)?;

            if sass_map.get_ref(&key).is_some() {
                return Err(("Duplicate key.", key_span).into());
            }

            sass_map.insert(
                Spanned {
                    node: key,
                    span: key_span,
                },
                value,
            );
        }

        Ok(Value::Map(sass_map))
    }

    /// Whether `node` can be a component of a slash-separated number (dart-sass's
    /// `_operandAllowsSlash`): a function call only when it is a calculation.
    pub(super) fn operand_allows_slash(&self, node: &AstExpr) -> bool {
        match node {
            AstExpr::FunctionCall(call) => {
                call.namespace.is_none()
                    && matches!(
                        call.name.as_str().to_ascii_lowercase().as_str(),
                        "calc"
                            | "clamp"
                            | "hypot"
                            | "sin"
                            | "cos"
                            | "tan"
                            | "asin"
                            | "acos"
                            | "atan"
                            | "sqrt"
                            | "exp"
                            | "sign"
                            | "mod"
                            | "rem"
                            | "atan2"
                            | "pow"
                            | "log"
                            | "calc-size"
                    )
                    && matches!(self.env.get_fn(call.name, None), Ok(None))
            }
            _ => true,
        }
    }

    pub(super) fn visit_bin_op(
        &mut self,
        lhs: AstExpr,
        op: BinaryOp,
        rhs: AstExpr,
        allows_slash: bool,
        span: Span,
    ) -> SassResult<Value> {
        // dart-sass decides at parse time whether operands may form a slash-separated number,
        // except for function calls, which may turn out to be calculations.
        if self.is_plain_css && !matches!(op, BinaryOp::SingleEq | BinaryOp::Div) {
            return Err(("Operators aren't allowed in plain CSS.", span).into());
        }

        let allows_slash =
            allows_slash && self.operand_allows_slash(&lhs) && self.operand_allows_slash(&rhs);
        let left = self.visit_expr(lhs)?;

        Ok(match op {
            BinaryOp::SingleEq => {
                let right = self.visit_expr(rhs)?;
                single_eq(&left, &right, self.options, span)?
            }
            BinaryOp::Or => {
                if left.is_truthy() {
                    left
                } else {
                    self.visit_expr(rhs)?
                }
            }
            BinaryOp::And => {
                if left.is_truthy() {
                    self.visit_expr(rhs)?
                } else {
                    left
                }
            }
            BinaryOp::Equal => {
                let right = self.visit_expr(rhs)?;
                Value::bool(left == right)
            }
            BinaryOp::NotEqual => {
                let right = self.visit_expr(rhs)?;
                Value::bool(left != right)
            }
            BinaryOp::GreaterThan
            | BinaryOp::GreaterThanEqual
            | BinaryOp::LessThan
            | BinaryOp::LessThanEqual => {
                let right = self.visit_expr(rhs)?;
                cmp(&left, &right, self.options, span, op)?
            }
            BinaryOp::Plus => {
                let right = self.visit_expr(rhs)?;
                add(left, right, self.options, span)?
            }
            BinaryOp::Minus => {
                let right = self.visit_expr(rhs)?;
                sub(left, right, self.options, span)?
            }
            BinaryOp::Mul => {
                let right = self.visit_expr(rhs)?;
                mul(left, right, self.options, span)?
            }
            BinaryOp::Div => {
                let right = self.visit_expr(rhs)?;

                let left_is_number = matches!(left, Value::Dimension { .. });
                let right_is_number = matches!(right, Value::Dimension { .. });

                if left_is_number && right_is_number && allows_slash {
                    let result = div(left.clone(), right.clone(), self.options, span)?;
                    return result.with_slash(
                        left.assert_number(span)?,
                        right.assert_number(span)?,
                        span,
                    );
                } else if left_is_number && right_is_number {
                    // todo: emit warning here. it prints too frequently, so we do not currently
                    // self.emit_warning(
                    //     Cow::Borrowed(format!(
                    //         "Using / for division outside of calc() is deprecated"
                    //     )),
                    //     span,
                    // );
                }

                div(left, right, self.options, span)?
            }
            BinaryOp::Rem => {
                let right = self.visit_expr(rhs)?;
                rem(left, right, self.options, span)?
            }
        })
    }
}
