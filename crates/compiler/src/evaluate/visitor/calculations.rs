//! Calculations, as dart-sass evaluates them.

use super::*;

impl<'a> Visitor<'a> {
    /// Evaluates `func_call` as a calculation (dart-sass's `_visitCalculation`).
    /// `in_legacy_sass_function` is the name of the global Sass function the call could have
    /// been (`min`, `max`, `round`, `abs`), which allows unitless numbers to mix with numbers
    /// with units, for backwards compatibility.
    pub(super) fn visit_calculation(
        &mut self,
        func_call: &FunctionCallExpr,
        in_legacy_sass_function: Option<&str>,
    ) -> SassResult<Value> {
        let span = func_call.span;
        let arguments = &func_call.arguments;
        if !arguments.named.is_empty() {
            return Err(("Keyword arguments can't be used with calculations.", span).into());
        } else if arguments.rest.is_some() {
            return Err(("Rest arguments can't be used with calculations.", span).into());
        }

        let lower = func_call.name.as_str().to_ascii_lowercase();
        let name = CalculationName::from_lowercase(&lower)
            .unwrap_or_else(|| unreachable!("unknown calculation name {lower}"));
        Self::check_calculation_arguments(name, arguments.positional.len(), span)?;

        let mut args = arguments
            .positional
            .iter()
            .map(|arg| self.visit_calculation_value(arg, in_legacy_sass_function, span))
            .collect::<SassResult<Vec<_>>>()?;

        if self.flags.in_supports_declaration() {
            return Ok(Value::Calculation(SassCalculation::unsimplified(
                name, args,
            )));
        }

        let options = self.options;
        let mut take = || (!args.is_empty()).then(|| args.remove(0));
        let first = take().unwrap();
        match name {
            CalculationName::Calc => Ok(SassCalculation::calc(first)),
            CalculationName::Sqrt
            | CalculationName::Sin
            | CalculationName::Cos
            | CalculationName::Tan
            | CalculationName::Asin
            | CalculationName::Acos
            | CalculationName::Atan => SassCalculation::single_argument(name, first, options, span),
            CalculationName::Abs => {
                let mut warnings = Vec::new();
                let value = SassCalculation::abs(first, &mut |m: &str| warnings.push(m.to_owned()));
                for warning in warnings {
                    self.emit_warning(&warning, span);
                }
                Ok(value)
            }
            CalculationName::Exp => SassCalculation::exp(first, options, span),
            CalculationName::Sign => Ok(SassCalculation::sign(first)),
            CalculationName::Min | CalculationName::Max | CalculationName::Hypot => {
                let rest = std::iter::from_fn(take);
                let all: Vec<_> = std::iter::once(first).chain(rest).collect();
                match name {
                    CalculationName::Min => SassCalculation::min(all, options, span),
                    CalculationName::Max => SassCalculation::max(all, options, span),
                    _ => SassCalculation::hypot(all, options, span),
                }
            }
            CalculationName::Pow => SassCalculation::pow(first, take(), options, span),
            CalculationName::Atan2 => SassCalculation::atan2(first, take(), options, span),
            CalculationName::Log => SassCalculation::log(first, take(), options, span),
            CalculationName::Mod => SassCalculation::modulus(first, take(), options, span),
            CalculationName::Rem => SassCalculation::rem(first, take(), options, span),
            CalculationName::Round => {
                let (second, third) = (take(), take());
                let mut warnings = Vec::new();
                let value = SassCalculation::round(
                    first,
                    second,
                    third,
                    in_legacy_sass_function,
                    options,
                    span,
                    &mut |m: &str| warnings.push(m.to_owned()),
                );
                for warning in warnings {
                    self.emit_warning(&warning, span);
                }
                value
            }
            CalculationName::Clamp => {
                let (value, max) = (take(), take());
                SassCalculation::clamp(first, value, max, options, span)
            }
            CalculationName::CalcSize => SassCalculation::calc_size(first, take(), span),
        }
    }

    /// Verifies that a calculation has the right number of arguments.
    pub(super) fn check_calculation_arguments(
        name: CalculationName,
        len: usize,
        span: Span,
    ) -> SassResult<()> {
        let max_args = match name {
            CalculationName::Calc
            | CalculationName::Sqrt
            | CalculationName::Sin
            | CalculationName::Cos
            | CalculationName::Tan
            | CalculationName::Asin
            | CalculationName::Acos
            | CalculationName::Atan
            | CalculationName::Abs
            | CalculationName::Exp
            | CalculationName::Sign => Some(1),
            CalculationName::Min | CalculationName::Max | CalculationName::Hypot => None,
            CalculationName::Pow
            | CalculationName::Atan2
            | CalculationName::Log
            | CalculationName::Mod
            | CalculationName::Rem
            | CalculationName::CalcSize => Some(2),
            CalculationName::Round | CalculationName::Clamp => Some(3),
        };
        if len == 0 {
            return Err(("Missing argument.", span).into());
        }
        if let Some(max) = max_args
            && len > max
        {
            let argument = if max == 1 { "argument" } else { "arguments" };
            let was = if len == 1 { "was" } else { "were" };
            return Err((
                format!("Only {max} {argument} allowed, but {len} {was} passed."),
                span,
            )
                .into());
        }
        Ok(())
    }

    /// Evaluates `expr` as an argument of a calculation (dart-sass's
    /// `_visitCalculationExpression`).
    pub(super) fn visit_calculation_value(
        &mut self,
        expr: &AstExpr,
        in_legacy_sass_function: Option<&str>,
        span: Span,
    ) -> SassResult<CalculationArg> {
        Ok(match expr {
            AstExpr::Paren(inner) => {
                match self.visit_calculation_value(inner, in_legacy_sass_function, span)? {
                    CalculationArg::String(text) => CalculationArg::String(format!("({text})")),
                    result => result,
                }
            }
            AstExpr::String(StringExpr(text, QuoteKind::None), ..)
                if expr.is_calculation_safe() =>
            {
                let constant = text.as_plain().and_then(|plain| {
                    Some(match plain.to_ascii_lowercase().as_str() {
                        "pi" => std::f64::consts::PI,
                        "e" => std::f64::consts::E,
                        "infinity" => f64::INFINITY,
                        "-infinity" => f64::NEG_INFINITY,
                        "nan" => f64::NAN,
                        _ => return None,
                    })
                });
                match constant {
                    Some(n) => CalculationArg::Number(SassNumber::new_unitless(n)),
                    None => {
                        CalculationArg::String(self.perform_interpolation(text.clone(), false)?)
                    }
                }
            }
            AstExpr::BinaryOp(binop) => {
                if matches!(binop.op, BinaryOp::Plus | BinaryOp::Minus)
                    && !binop.whitespace_around_operator
                {
                    return Err((
                        "\"+\" and \"-\" must be surrounded by whitespace in calculations.",
                        binop.span,
                    )
                        .into());
                }
                if !matches!(
                    binop.op,
                    BinaryOp::Plus | BinaryOp::Minus | BinaryOp::Mul | BinaryOp::Div
                ) {
                    return Err(
                        ("This operation can't be used in a calculation.", binop.span).into(),
                    );
                }
                let lhs =
                    self.visit_calculation_value(&binop.lhs, in_legacy_sass_function, span)?;
                let rhs =
                    self.visit_calculation_value(&binop.rhs, in_legacy_sass_function, span)?;
                let mut warnings = Vec::new();
                let result = SassCalculation::operate_internal(
                    binop.op,
                    lhs,
                    rhs,
                    in_legacy_sass_function,
                    !self.flags.in_supports_declaration(),
                    self.options,
                    binop.span,
                    &mut |m: &str| warnings.push(m.to_owned()),
                );
                for warning in warnings {
                    self.emit_warning(&warning, binop.span);
                }
                result?
            }
            // In plain CSS, function calls are `InterpolatedFunction`s whose names have no
            // interpolation; dart-sass parses them as function calls.
            AstExpr::InterpolatedFunction(func)
                if self.is_plain_css && func.name.as_plain().is_some() =>
            {
                match self.visit_expr(expr.clone())? {
                    Value::String(s, QuoteKind::None) => CalculationArg::String(s),
                    value => {
                        return Err((
                            format!(
                                "Value {} can't be used in a calculation.",
                                value.inspect(span)?
                            ),
                            span,
                        )
                            .into());
                    }
                }
            }
            AstExpr::Number { .. }
            | AstExpr::Variable { .. }
            | AstExpr::FunctionCall(..)
            | AstExpr::LegacyIf(..) => match self.visit_expr(expr.clone())? {
                Value::Dimension(n) => CalculationArg::Number(n),
                Value::Calculation(calc) => CalculationArg::Calculation(calc),
                Value::String(s, QuoteKind::None) => CalculationArg::String(s),
                value => {
                    return Err((
                        format!(
                            "Value {} can't be used in a calculation.",
                            value.inspect(span)?
                        ),
                        span,
                    )
                        .into());
                }
            },
            AstExpr::List(list)
                if list.separator == ListSeparator::Space
                    && list.brackets == Brackets::None
                    && list.elems.len() > 1 =>
            {
                let mut elements = list
                    .elems
                    .iter()
                    .map(|elem| {
                        self.visit_calculation_value(&elem.node, in_legacy_sass_function, span)
                    })
                    .collect::<SassResult<Vec<_>>>()?;

                Self::check_adjacent_calculation_values(&elements, list)?;

                for (element, node) in elements.iter_mut().zip(&list.elems) {
                    if matches!(element, CalculationArg::Operation { .. })
                        && matches!(node.node, AstExpr::Paren(..))
                    {
                        let text = serialize_calculation_arg(element, self.options, span)?;
                        *element = CalculationArg::String(format!("({text})"));
                    }
                }

                CalculationArg::String(
                    elements
                        .iter()
                        .map(|element| serialize_calculation_arg(element, self.options, span))
                        .collect::<SassResult<Vec<_>>>()?
                        .join(" "),
                )
            }
            _ => {
                // dart-sass counts CSS `if()` as calculation-safe, but can't evaluate it here.
                debug_assert!(!expr.is_calculation_safe() || matches!(expr, AstExpr::If(..)));
                return Err(("This expression can't be used in a calculation.", span).into());
            }
        })
    }

    /// Throws if two adjacent values of a space-separated list in a calculation are both not
    /// strings.
    pub(super) fn check_adjacent_calculation_values(
        elements: &[CalculationArg],
        list: &ListExpr,
    ) -> SassResult<()> {
        for i in 1..elements.len() {
            if matches!(elements[i - 1], CalculationArg::String(..))
                || matches!(elements[i], CalculationArg::String(..))
            {
                continue;
            }
            let current = &list.elems[i];
            let looks_unary = match &current.node {
                AstExpr::UnaryOp(UnaryOp::Neg | UnaryOp::Plus, ..) => true,
                AstExpr::Number { n, .. } => n.0 < 0.0,
                _ => false,
            };
            // `calc(1 -2)` parses as a list whose second value is a negative number.
            if looks_unary {
                return Err((
                    "\"+\" and \"-\" must be surrounded by whitespace in calculations.",
                    current.span,
                )
                    .into());
            }
            return Err((
                "Missing math operator.",
                list.elems[i - 1].span.merge(current.span),
            )
                .into());
        }
        Ok(())
    }
}
