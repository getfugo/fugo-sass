//! Stepped-value functions: `round()`, `mod()` and `rem()`, and `calc-size()`.

use super::*;

impl SassCalculation {
    pub fn rem(
        dividend: CalculationArg,
        modulus: Option<CalculationArg>,
        options: &Options,
        span: Span,
    ) -> SassResult<Value> {
        let (args, numbers) = Self::two_compatible(dividend, modulus, options, span)?;
        let (dividend, modulus) = match numbers {
            Some(numbers) => numbers,
            None => return Ok(Self::value(CalculationName::Rem, args)),
        };
        let result = Self::modulo(&dividend, &modulus);
        if sign_including_zero(modulus.num.0) != sign_including_zero(dividend.num.0) {
            if modulus.num.0.is_infinite() {
                return Ok(Value::Dimension(dividend));
            }
            if result.num.0 == 0.0 {
                return Ok(Value::Dimension(SassNumber {
                    num: Number(-result.num.0),
                    ..result
                }));
            }
            return Ok(Value::Dimension(result - modulus));
        }
        Ok(Value::Dimension(result))
    }

    pub fn modulus(
        dividend: CalculationArg,
        modulus: Option<CalculationArg>,
        options: &Options,
        span: Span,
    ) -> SassResult<Value> {
        let (args, numbers) = Self::two_compatible(dividend, modulus, options, span)?;
        Ok(match numbers {
            Some((dividend, modulus)) => Value::Dimension(Self::modulo(&dividend, &modulus)),
            None => Self::value(CalculationName::Mod, args),
        })
    }

    /// The simplified arguments of `rem()` or `mod()`, and the two numbers when they have
    /// compatible units.
    #[allow(clippy::type_complexity)]
    pub(super) fn two_compatible(
        left: CalculationArg,
        right: Option<CalculationArg>,
        options: &Options,
        span: Span,
    ) -> SassResult<(Vec<CalculationArg>, Option<(SassNumber, SassNumber)>)> {
        let left = Self::simplify(left);
        let right = right.map(Self::simplify);
        let args: Vec<CalculationArg> = std::iter::once(left).chain(right).collect();
        Self::verify_length(&args, 2, span)?;
        Self::verify_compatible_numbers(&args, options, span)?;
        if let [CalculationArg::Number(l), CalculationArg::Number(r)] = args.as_slice()
            && l.has_compatible_units(&r.unit)
        {
            let numbers = (l.clone(), r.clone());
            return Ok((args, Some(numbers)));
        }
        Ok((args, None))
    }

    /// dart-sass's `SassNumber.modulo`: `right` converted to `left`'s units.
    pub(super) fn modulo(left: &SassNumber, right: &SassNumber) -> SassNumber {
        let right = right.num.convert(&right.unit, &left.unit);
        SassNumber {
            num: left.num % right,
            unit: left.unit.clone(),
            as_slash: None,
        }
    }

    /// `round()`. `in_legacy_sass_function` is the name of the global function the call
    /// could have been (`round`), which allows a single number with units.
    pub fn round(
        strategy_or_number: CalculationArg,
        number_or_step: Option<CalculationArg>,
        step: Option<CalculationArg>,
        in_legacy_sass_function: Option<&str>,
        options: &Options,
        span: Span,
        warn: Warn<'_>,
    ) -> SassResult<Value> {
        let first = Self::simplify(strategy_or_number);
        let second = number_or_step.map(Self::simplify);
        let third = step.map(Self::simplify);

        let strategy = |arg: &CalculationArg| match arg {
            CalculationArg::String(s)
                if matches!(s.as_str(), "nearest" | "up" | "down" | "to-zero") =>
            {
                Some(s.clone())
            }
            _ => None,
        };

        match (first, second, third) {
            // dart-sass rounds a single number to an `int`, which fails for NaN and infinities.
            (CalculationArg::Number(number), None, None)
                if !number.num.0.is_finite()
                    && (number.unit == Unit::None || in_legacy_sass_function.is_some()) =>
            {
                Err(("Infinity or NaN toInt", span).into())
            }
            (CalculationArg::Number(number), None, None) if number.unit == Unit::None => Ok(
                Value::Dimension(SassNumber::new_unitless(number.num.0.round())),
            ),
            (CalculationArg::Number(number), None, None) if in_legacy_sass_function.is_some() => {
                warn(
                    "In future versions of Sass, round() will be interpreted as a CSS round() \
                     calculation. This requires an explicit modulus when rounding numbers with \
                     units. If you want to use the Sass function, call math.round() instead.\n\
                     \n\
                     See https://sass-lang.com/d/import",
                );
                Ok(Value::Dimension(SassNumber {
                    num: Number(number.num.0.round()),
                    unit: number.unit,
                    as_slash: None,
                }))
            }
            (CalculationArg::Number(number), Some(CalculationArg::Number(step)), None) => {
                let args = [
                    CalculationArg::Number(number.clone()),
                    CalculationArg::Number(step.clone()),
                ];
                Self::verify_compatible_numbers(&args, options, span)?;
                if !number.has_compatible_units(&step.unit) {
                    return Ok(Self::value(CalculationName::Round, args.to_vec()));
                }
                Ok(Value::Dimension(Self::round_with_step(
                    "nearest", &number, &step,
                )))
            }
            (first, Some(CalculationArg::Number(number)), Some(CalculationArg::Number(step)))
                if strategy(&first).is_some() =>
            {
                let strategy_name = strategy(&first).unwrap();
                let numbers = [
                    CalculationArg::Number(number.clone()),
                    CalculationArg::Number(step.clone()),
                ];
                Self::verify_compatible_numbers(&numbers, options, span)?;
                if !number.has_compatible_units(&step.unit) {
                    let [number, step] = numbers;
                    return Ok(Self::value(
                        CalculationName::Round,
                        vec![first, number, step],
                    ));
                }
                Ok(Value::Dimension(Self::round_with_step(
                    &strategy_name,
                    &number,
                    &step,
                )))
            }
            (first, Some(rest @ CalculationArg::String(..)), None)
                if strategy(&first).is_some() =>
            {
                Ok(Self::value(CalculationName::Round, vec![first, rest]))
            }
            (first, Some(_), None) if strategy(&first).is_some() => {
                Err(("If strategy is not null, step is required.", span).into())
            }
            (first, None, None) if strategy(&first).is_some() => {
                Err(("Number to round and step arguments are required.", span).into())
            }
            (number, None, None) => Ok(Self::value(CalculationName::Round, vec![number])),
            (number, Some(step), None) => {
                Ok(Self::value(CalculationName::Round, vec![number, step]))
            }
            (first, Some(number), Some(step))
                if strategy(&first).is_some() || is_special_variable(&first) =>
            {
                Ok(Self::value(
                    CalculationName::Round,
                    vec![first, number, step],
                ))
            }
            (first, Some(_), Some(_)) => Err((
                format!(
                    "{} must be either nearest, up, down or to-zero.",
                    Self::describe(&first, options, span)?
                ),
                span,
            )
                .into()),
            (_, None, Some(_)) => Err(("Invalid parameters.", span).into()),
        }
    }

    /// `number` rounded with `strategy` to a multiple of `step`.
    pub(super) fn round_with_step(
        strategy: &str,
        number: &SassNumber,
        step: &SassNumber,
    ) -> SassNumber {
        let matching = |value: f64| SassNumber {
            num: Number(value),
            unit: number.unit.clone(),
            as_slash: None,
        };
        let (value, step_value) = (number.num.0, step.num.0);

        if (value.is_infinite() && step_value.is_infinite())
            || step_value == 0.0
            || value.is_nan()
            || step_value.is_nan()
        {
            return matching(f64::NAN);
        }
        if value.is_infinite() {
            return number.clone();
        }

        if step_value.is_infinite() {
            return match (strategy, value) {
                (_, 0.0) => number.clone(),
                ("nearest" | "to-zero", v) if v > 0.0 => matching(0.0),
                ("nearest" | "to-zero", _) => matching(-0.0),
                ("up", v) if v > 0.0 => matching(f64::INFINITY),
                ("up", _) => matching(-0.0),
                ("down", v) if v < 0.0 => matching(f64::NEG_INFINITY),
                _ => matching(0.0),
            };
        }

        let step = step.num.convert(&step.unit, &number.unit).0;
        let ratio = value / step;
        matching(match strategy {
            "nearest" => ratio.round() * step,
            "up" if step_value < 0.0 => ratio.floor() * step,
            "up" => ratio.ceil() * step,
            "down" if step_value < 0.0 => ratio.ceil() * step,
            "down" => ratio.floor() * step,
            "to-zero" if value < 0.0 => ratio.ceil() * step,
            "to-zero" => ratio.floor() * step,
            _ => f64::NAN,
        })
    }

    /// `calc-size()`: always a calculation.
    pub fn calc_size(
        basis: CalculationArg,
        value: Option<CalculationArg>,
        span: Span,
    ) -> SassResult<Value> {
        let args: Vec<CalculationArg> = std::iter::once(basis.clone())
            .chain(value.clone())
            .collect();
        Self::verify_length(&args, 2, span)?;
        let args = std::iter::once(Self::simplify(basis))
            .chain(value.map(Self::simplify))
            .collect();
        Ok(Self::value(CalculationName::CalcSize, args))
    }
}
