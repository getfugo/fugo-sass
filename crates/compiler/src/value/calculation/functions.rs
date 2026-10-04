//! The CSS math functions: `min()`, `max()`, `hypot()`, `clamp()`, `pow()`, `log()`, trigonometry and the like.

use super::*;

impl SassCalculation {
    pub fn min(args: Vec<CalculationArg>, options: &Options, span: Span) -> SassResult<Value> {
        Self::min_or_max(CalculationName::Min, args, options, span)
    }

    pub fn max(args: Vec<CalculationArg>, options: &Options, span: Span) -> SassResult<Value> {
        Self::min_or_max(CalculationName::Max, args, options, span)
    }

    pub(super) fn min_or_max(
        name: CalculationName,
        args: Vec<CalculationArg>,
        options: &Options,
        span: Span,
    ) -> SassResult<Value> {
        let args = Self::simplify_arguments(args);
        if args.is_empty() {
            return Err((format!("{name}() must have at least one argument."), span).into());
        }

        let mut extremum: Option<&SassNumber> = None;
        for arg in &args {
            match arg {
                CalculationArg::Number(n) if extremum.is_none_or(|e| e.is_comparable_to(n)) => {
                    let replace = match extremum {
                        None => true,
                        Some(e) => {
                            let other = n.num.convert(&n.unit, &e.unit);
                            if name == CalculationName::Min {
                                e.num > other
                            } else {
                                e.num < other
                            }
                        }
                    };
                    if replace {
                        extremum = Some(n);
                    }
                }
                _ => {
                    extremum = None;
                    break;
                }
            }
        }
        if let Some(n) = extremum {
            return Ok(Value::Dimension(n.clone()));
        }

        Self::verify_compatible_numbers(&args, options, span)?;
        Ok(Self::value(name, args))
    }

    pub fn hypot(args: Vec<CalculationArg>, options: &Options, span: Span) -> SassResult<Value> {
        let args = Self::simplify_arguments(args);
        if args.is_empty() {
            return Err(("hypot() must have at least one argument.", span).into());
        }
        Self::verify_compatible_numbers(&args, options, span)?;

        let first = match &args[0] {
            CalculationArg::Number(n) if n.unit != Unit::Percent => n,
            _ => return Ok(Self::value(CalculationName::Hypot, args)),
        };
        let mut subtotal = 0.0;
        for arg in &args {
            match arg {
                CalculationArg::Number(n) if n.has_compatible_units(&first.unit) => {
                    let value = n.num.convert(&n.unit, &first.unit).0;
                    subtotal += value * value;
                }
                _ => return Ok(Self::value(CalculationName::Hypot, args)),
            }
        }
        Ok(Value::Dimension(SassNumber {
            num: Number(subtotal.sqrt()),
            unit: first.unit.clone(),
            as_slash: None,
        }))
    }

    /// `sqrt()`, `sin()`, `cos()`, `tan()`, `asin()`, `acos()` and `atan()`.
    pub fn single_argument(
        name: CalculationName,
        arg: CalculationArg,
        options: &Options,
        span: Span,
    ) -> SassResult<Value> {
        let number = match Self::simplify(arg) {
            CalculationArg::Number(n) => n,
            arg => return Ok(Self::value(name, vec![arg])),
        };
        let forbid_units = matches!(
            name,
            CalculationName::Sqrt
                | CalculationName::Atan
                | CalculationName::Asin
                | CalculationName::Acos
        );
        if forbid_units {
            Self::assert_no_units(&number, None, options, span)?;
        }
        Ok(Value::Dimension(match name {
            CalculationName::Sqrt => SassNumber::new_unitless(number.num.0.sqrt()),
            CalculationName::Sin => {
                SassNumber::new_unitless(Self::to_radians(&number, options, span)?.sin())
            }
            CalculationName::Cos => {
                SassNumber::new_unitless(Self::to_radians(&number, options, span)?.cos())
            }
            CalculationName::Tan => {
                SassNumber::new_unitless(Self::to_radians(&number, options, span)?.tan())
            }
            CalculationName::Atan => Self::degrees(number.num.0.atan()),
            CalculationName::Asin => Self::degrees(number.num.0.asin()),
            CalculationName::Acos => Self::degrees(number.num.0.acos()),
            _ => unreachable!("{name} takes one argument"),
        }))
    }

    pub fn abs(arg: CalculationArg, warn: Warn<'_>) -> Value {
        match Self::simplify(arg) {
            CalculationArg::Number(n) => {
                if n.unit == Unit::Percent {
                    warn(&format!(
                        "Passing percentage units to the global abs() function is deprecated.\n\
                         In the future, this will emit a CSS abs() function to be resolved by the browser.\n\
                         To preserve current behavior: math.abs({0})\n\
                         To emit a CSS abs() now: abs(#{{{0}}})\n\
                         More info: https://sass-lang.com/d/abs-percent",
                        n.num.inspect() + &n.unit.to_string()
                    ));
                }
                Value::Dimension(SassNumber {
                    num: Number(n.num.0.abs()),
                    unit: n.unit,
                    as_slash: None,
                })
            }
            arg => Self::value(CalculationName::Abs, vec![arg]),
        }
    }

    pub fn exp(arg: CalculationArg, options: &Options, span: Span) -> SassResult<Value> {
        match Self::simplify(arg) {
            CalculationArg::Number(n) => {
                Self::assert_no_units(&n, None, options, span)?;
                Ok(Value::Dimension(SassNumber::new_unitless(
                    consts::E.powf(n.num.0),
                )))
            }
            arg => Ok(Self::value(CalculationName::Exp, vec![arg])),
        }
    }

    pub fn sign(arg: CalculationArg) -> Value {
        match Self::simplify(arg) {
            CalculationArg::Number(n) if n.num.0.is_nan() || n.num.0 == 0.0 => Value::Dimension(n),
            CalculationArg::Number(n) if n.unit != Unit::Percent => Value::Dimension(SassNumber {
                num: Number(n.num.0.signum()),
                unit: n.unit,
                as_slash: None,
            }),
            arg => Self::value(CalculationName::Sign, vec![arg]),
        }
    }

    pub fn clamp(
        min: CalculationArg,
        value: Option<CalculationArg>,
        max: Option<CalculationArg>,
        options: &Options,
        span: Span,
    ) -> SassResult<Value> {
        if value.is_none() && max.is_some() {
            return Err(("If value is null, max must also be null.", span).into());
        }

        let min = Self::simplify(min);
        let value = value.map(Self::simplify);
        let max = max.map(Self::simplify);

        if let (
            CalculationArg::Number(min),
            Some(CalculationArg::Number(value)),
            Some(CalculationArg::Number(max)),
        ) = (&min, &value, &max)
            && min.has_compatible_units(&value.unit)
            && min.has_compatible_units(&max.unit)
        {
            if value.num <= min.num.convert(&min.unit, &value.unit) {
                return Ok(Value::Dimension(min.clone()));
            }
            if value.num >= max.num.convert(&max.unit, &value.unit) {
                return Ok(Value::Dimension(max.clone()));
            }
            return Ok(Value::Dimension(value.clone()));
        }

        let args: Vec<CalculationArg> = std::iter::once(min).chain(value).chain(max).collect();
        Self::verify_compatible_numbers(&args, options, span)?;
        Self::verify_length(&args, 3, span)?;
        Ok(Self::value(CalculationName::Clamp, args))
    }

    pub fn pow(
        base: CalculationArg,
        exponent: Option<CalculationArg>,
        options: &Options,
        span: Span,
    ) -> SassResult<Value> {
        let args: Vec<CalculationArg> = std::iter::once(base.clone())
            .chain(exponent.clone())
            .collect();
        Self::verify_length(&args, 2, span)?;
        match (Self::simplify(base), exponent.map(Self::simplify)) {
            (CalculationArg::Number(base), Some(CalculationArg::Number(exponent))) => {
                Self::assert_no_units(&base, None, options, span)?;
                Self::assert_no_units(&exponent, None, options, span)?;
                Ok(Value::Dimension(SassNumber::new_unitless(
                    base.num.0.powf(exponent.num.0),
                )))
            }
            // dart-sass keeps the arguments as they were passed here.
            _ => Ok(Self::value(CalculationName::Pow, args)),
        }
    }

    pub fn log(
        number: CalculationArg,
        base: Option<CalculationArg>,
        options: &Options,
        span: Span,
    ) -> SassResult<Value> {
        let number = Self::simplify(number);
        let base = base.map(Self::simplify);
        match (&number, &base) {
            (CalculationArg::Number(n), None) => {
                Self::assert_no_units(n, None, options, span)?;
                Ok(Value::Dimension(SassNumber::new_unitless(n.num.0.ln())))
            }
            (CalculationArg::Number(n), Some(CalculationArg::Number(b))) => {
                Self::assert_no_units(n, None, options, span)?;
                Self::assert_no_units(b, None, options, span)?;
                Ok(Value::Dimension(SassNumber::new_unitless(
                    n.num.0.ln() / b.num.0.ln(),
                )))
            }
            _ => Ok(Self::value(
                CalculationName::Log,
                std::iter::once(number).chain(base).collect(),
            )),
        }
    }

    pub fn atan2(
        y: CalculationArg,
        x: Option<CalculationArg>,
        options: &Options,
        span: Span,
    ) -> SassResult<Value> {
        let y = Self::simplify(y);
        let x = x.map(Self::simplify);
        let args: Vec<CalculationArg> = std::iter::once(y).chain(x).collect();
        Self::verify_length(&args, 2, span)?;
        Self::verify_compatible_numbers(&args, options, span)?;
        if let [CalculationArg::Number(y), CalculationArg::Number(x)] = args.as_slice()
            && y.unit != Unit::Percent
            && x.unit != Unit::Percent
            && y.has_compatible_units(&x.unit)
        {
            let x = x.num.convert(&x.unit, &y.unit).0;
            return Ok(Value::Dimension(Self::degrees(y.num.0.atan2(x))));
        }
        Ok(Self::value(CalculationName::Atan2, args))
    }
}
