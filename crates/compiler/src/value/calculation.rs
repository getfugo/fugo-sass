//! Calculations: `calc()`, `min()`, `max()`, `clamp()` and the CSS Values 4 functions
//! (`round()`, `mod()`, `sin()`, …), ported from dart-sass's `SassCalculation`
//! (`lib/src/value/calculation.dart`, dart-sass 1.105.1).

use core::fmt;
use std::{f64::consts, iter::Iterator};

use codemap::Span;

use crate::{
    common::BinaryOp,
    error::SassResult,
    serializer::inspect_number,
    unit::Unit,
    value::{conversion_factor, fuzzy_less_than, sign_including_zero, Number, SassNumber, Value},
    Options,
};

/// An argument of a calculation: a number, a calculation, an unquoted string (interpolation
/// included, parenthesized) or an operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalculationArg {
    Number(SassNumber),
    Calculation(SassCalculation),
    String(String),
    Operation {
        lhs: Box<Self>,
        op: BinaryOp,
        rhs: Box<Self>,
    },
}

impl CalculationArg {
    /// Whether the right-hand operation of a calculation should be parenthesized: in
    /// `a ? (b # c)`, `outer` is `?` and `right` is `#`.
    pub fn parenthesize_calculation_rhs(outer: BinaryOp, right: BinaryOp) -> bool {
        if outer == BinaryOp::Div {
            true
        } else if outer == BinaryOp::Plus {
            false
        } else {
            right == BinaryOp::Plus || right == BinaryOp::Minus
        }
    }

    fn is_string(&self) -> bool {
        matches!(self, Self::String(..))
    }
}

/// The calculations Sass knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CalculationName {
    Calc,
    Min,
    Max,
    Clamp,
    Hypot,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Asin,
    Acos,
    Atan,
    Atan2,
    Abs,
    Exp,
    Sign,
    Pow,
    Log,
    Mod,
    Rem,
    Round,
    CalcSize,
}

impl CalculationName {
    /// The calculation `name` (lowercase) is, if any.
    pub(crate) fn from_lowercase(name: &str) -> Option<Self> {
        Some(match name {
            "calc" => Self::Calc,
            "min" => Self::Min,
            "max" => Self::Max,
            "clamp" => Self::Clamp,
            "hypot" => Self::Hypot,
            "sqrt" => Self::Sqrt,
            "sin" => Self::Sin,
            "cos" => Self::Cos,
            "tan" => Self::Tan,
            "asin" => Self::Asin,
            "acos" => Self::Acos,
            "atan" => Self::Atan,
            "atan2" => Self::Atan2,
            "abs" => Self::Abs,
            "exp" => Self::Exp,
            "sign" => Self::Sign,
            "pow" => Self::Pow,
            "log" => Self::Log,
            "mod" => Self::Mod,
            "rem" => Self::Rem,
            "round" => Self::Round,
            "calc-size" => Self::CalcSize,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Calc => "calc",
            Self::Min => "min",
            Self::Max => "max",
            Self::Clamp => "clamp",
            Self::Hypot => "hypot",
            Self::Sqrt => "sqrt",
            Self::Sin => "sin",
            Self::Cos => "cos",
            Self::Tan => "tan",
            Self::Asin => "asin",
            Self::Acos => "acos",
            Self::Atan => "atan",
            Self::Atan2 => "atan2",
            Self::Abs => "abs",
            Self::Exp => "exp",
            Self::Sign => "sign",
            Self::Pow => "pow",
            Self::Log => "log",
            Self::Mod => "mod",
            Self::Rem => "rem",
            Self::Round => "round",
            Self::CalcSize => "calc-size",
        }
    }
}

impl fmt::Display for CalculationName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SassCalculation {
    pub name: CalculationName,
    pub args: Vec<CalculationArg>,
}

/// Reports a deprecation warning (the message) during a calculation.
pub(crate) type Warn<'a> = &'a mut dyn FnMut(&str);

impl SassCalculation {
    /// A calculation that is not simplified (in `@supports` declarations).
    pub fn unsimplified(name: CalculationName, args: Vec<CalculationArg>) -> Self {
        Self { name, args }
    }

    /// The calculation `name(args)` as a value.
    fn value(name: CalculationName, args: Vec<CalculationArg>) -> Value {
        Value::Calculation(Self { name, args })
    }

    pub fn calc(arg: CalculationArg) -> Value {
        match Self::simplify(arg) {
            CalculationArg::Number(n) => Value::Dimension(n),
            CalculationArg::Calculation(c) => Value::Calculation(c),
            simplified => Self::value(CalculationName::Calc, vec![simplified]),
        }
    }

    pub fn min(args: Vec<CalculationArg>, options: &Options, span: Span) -> SassResult<Value> {
        Self::min_or_max(CalculationName::Min, args, options, span)
    }

    pub fn max(args: Vec<CalculationArg>, options: &Options, span: Span) -> SassResult<Value> {
        Self::min_or_max(CalculationName::Max, args, options, span)
    }

    fn min_or_max(
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
                CalculationArg::Number(n) if extremum.map_or(true, |e| e.is_comparable_to(n)) => {
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
        {
            if min.has_compatible_units(&value.unit) && min.has_compatible_units(&max.unit) {
                if value.num <= min.num.convert(&min.unit, &value.unit) {
                    return Ok(Value::Dimension(min.clone()));
                }
                if value.num >= max.num.convert(&max.unit, &value.unit) {
                    return Ok(Value::Dimension(max.clone()));
                }
                return Ok(Value::Dimension(value.clone()));
            }
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
        if let [CalculationArg::Number(y), CalculationArg::Number(x)] = args.as_slice() {
            if y.unit != Unit::Percent && x.unit != Unit::Percent && y.has_compatible_units(&x.unit)
            {
                let x = x.num.convert(&x.unit, &y.unit).0;
                return Ok(Value::Dimension(Self::degrees(y.num.0.atan2(x))));
            }
        }
        Ok(Self::value(CalculationName::Atan2, args))
    }

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
    fn two_compatible(
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
        if let [CalculationArg::Number(l), CalculationArg::Number(r)] = args.as_slice() {
            if l.has_compatible_units(&r.unit) {
                let numbers = (l.clone(), r.clone());
                return Ok((args, Some(numbers)));
            }
        }
        Ok((args, None))
    }

    /// dart-sass's `SassNumber.modulo`: `right` converted to `left`'s units.
    fn modulo(left: &SassNumber, right: &SassNumber) -> SassNumber {
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
    fn round_with_step(strategy: &str, number: &SassNumber, step: &SassNumber) -> SassNumber {
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
                (_, v) if v == 0.0 => number.clone(),
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

    /// An operation of a calculation, simplified unless `simplify` is false.
    /// `in_legacy_sass_function` is the name of the global function the call could have been
    /// (`min`, `max`, …), which allows unitless numbers to be added to numbers with units.
    #[allow(clippy::too_many_arguments)]
    pub fn operate_internal(
        mut op: BinaryOp,
        left: CalculationArg,
        right: CalculationArg,
        in_legacy_sass_function: Option<&str>,
        simplify: bool,
        options: &Options,
        span: Span,
        warn: Warn<'_>,
    ) -> SassResult<CalculationArg> {
        if !simplify {
            return Ok(CalculationArg::Operation {
                lhs: Box::new(left),
                op,
                rhs: Box::new(right),
            });
        }

        let left = Self::simplify(left);
        let mut right = Self::simplify(right);

        if op == BinaryOp::Plus || op == BinaryOp::Minus {
            if let (CalculationArg::Number(l), CalculationArg::Number(r)) = (&left, &right) {
                let mut compatible = l.has_compatible_units(&r.unit);
                if !compatible {
                    if let Some(name) = in_legacy_sass_function {
                        if l.is_comparable_to(r) {
                            warn(&format!(
                                "In future versions of Sass, {name}() will be interpreted as the \
                                 CSS {name}() calculation. This doesn't allow unitless numbers to \
                                 be mixed with numbers with units. If you want to use the Sass \
                                 function, call math.{name}() instead.\n\
                                 \n\
                                 See https://sass-lang.com/d/import"
                            ));
                            compatible = true;
                        }
                    }
                }
                if compatible {
                    let (l, r) = (l.clone(), r.clone());
                    return Ok(CalculationArg::Number(if op == BinaryOp::Plus {
                        l + r
                    } else {
                        l - r
                    }));
                }
            }

            Self::verify_compatible_numbers(&[left.clone(), right.clone()], options, span)?;

            if let CalculationArg::Number(n) = &mut right {
                if fuzzy_less_than(n.num.0, 0.0) {
                    n.num = -n.num;
                    op = if op == BinaryOp::Plus {
                        BinaryOp::Minus
                    } else {
                        BinaryOp::Plus
                    };
                }
            }

            return Ok(CalculationArg::Operation {
                lhs: Box::new(left),
                op,
                rhs: Box::new(right),
            });
        }

        Ok(match (left, right) {
            (CalculationArg::Number(l), CalculationArg::Number(r)) => {
                CalculationArg::Number(if op == BinaryOp::Mul { l * r } else { l / r })
            }
            (left, right) => CalculationArg::Operation {
                lhs: Box::new(left),
                op,
                rhs: Box::new(right),
            },
        })
    }

    /// Throws if `args` isn't `len` long *and* has no string.
    fn verify_length(args: &[CalculationArg], len: usize, span: Span) -> SassResult<()> {
        if args.len() == len || args.iter().any(CalculationArg::is_string) {
            return Ok(());
        }
        let was_or_were = if args.len() == 1 { "was" } else { "were" };
        Err((
            format!(
                "{len} arguments required, but only {} {was_or_were} passed.",
                args.len()
            ),
            span,
        )
            .into())
    }

    /// Throws if two numbers of `args` are known to be incompatible, or a number has units
    /// too complex for calculations.
    pub(crate) fn verify_compatible_numbers(
        args: &[CalculationArg],
        options: &Options,
        span: Span,
    ) -> SassResult<()> {
        for arg in args {
            if let CalculationArg::Number(n) = arg {
                if n.unit.is_complex() {
                    return Err((
                        format!(
                            "Number {} isn't compatible with CSS calculations.",
                            inspect_number(n, options, span)?
                        ),
                        span,
                    )
                        .into());
                }
            }
        }

        for (i, number1) in args.iter().enumerate() {
            let number1 = match number1 {
                CalculationArg::Number(n) => n,
                _ => continue,
            };
            for number2 in &args[i + 1..] {
                let number2 = match number2 {
                    CalculationArg::Number(n) => n,
                    _ => continue,
                };
                if number1.has_possibly_compatible_units(number2) {
                    continue;
                }
                return Err((
                    format!(
                        "{} and {} are incompatible.",
                        inspect_number(number1, options, span)?,
                        inspect_number(number2, options, span)?
                    ),
                    span,
                )
                    .into());
            }
        }

        Ok(())
    }

    fn assert_no_units(
        number: &SassNumber,
        name: Option<&str>,
        options: &Options,
        span: Span,
    ) -> SassResult<()> {
        if number.unit == Unit::None {
            return Ok(());
        }
        let prefix = name.map_or_else(String::new, |name| format!("${name}: "));
        Err((
            format!(
                "{prefix}Expected {} to have no units.",
                inspect_number(number, options, span)?
            ),
            span,
        )
            .into())
    }

    /// dart-sass's `coerceValueToUnit("rad", "number")`.
    fn to_radians(number: &SassNumber, options: &Options, span: Span) -> SassResult<f64> {
        if number.unit == Unit::None {
            return Ok(number.num.0);
        }
        match conversion_factor(&number.unit, &Unit::Rad) {
            Some(factor) => Ok(number.num.0 * factor),
            None => Err((
                format!(
                    "$number: Expected {} to have an angle unit (deg, grad, rad, turn).",
                    inspect_number(number, options, span)?
                ),
                span,
            )
                .into()),
        }
    }

    fn degrees(radians: f64) -> SassNumber {
        SassNumber {
            num: Number(radians * (180.0 / consts::PI)),
            unit: Unit::Deg,
            as_slash: None,
        }
    }

    fn describe(arg: &CalculationArg, options: &Options, span: Span) -> SassResult<String> {
        crate::serializer::serialize_calculation_arg(arg, options, span)
    }

    /// Simplifies a calculation argument: a nested `calc()` is unwrapped.
    fn simplify(arg: CalculationArg) -> CalculationArg {
        match arg {
            CalculationArg::Calculation(mut calc)
                if calc.name == CalculationName::Calc && calc.args.len() == 1 =>
            {
                match calc.args.remove(0) {
                    CalculationArg::String(text) if needs_parentheses(&text) => {
                        CalculationArg::String(format!("({text})"))
                    }
                    value => value,
                }
            }
            arg => arg,
        }
    }

    fn simplify_arguments(args: Vec<CalculationArg>) -> Vec<CalculationArg> {
        args.into_iter().map(Self::simplify).collect()
    }
}

/// dart-sass's `isSpecialVariable`: an unquoted `var(…)`, `attr(…)` or CSS `if(…)`.
fn is_special_variable(arg: &CalculationArg) -> bool {
    match arg {
        CalculationArg::String(s) => {
            let lower = s.to_ascii_lowercase();
            lower.starts_with("var(") || lower.starts_with("attr(") || lower.starts_with("if(")
        }
        _ => false,
    }
}

/// Whether `text`, the contents of a `calc()` embedded in another calculation, needs
/// parentheses.
fn needs_parentheses(text: &str) -> bool {
    let needs = |c: char| c.is_whitespace() || c == '/' || c == '*';
    let chars: Vec<char> = text.chars().collect();
    let Some(&first) = chars.first() else {
        return false;
    };
    if needs(first) {
        return true;
    }
    let mut could_be_var = chars.len() >= 4 && first.eq_ignore_ascii_case(&'v');

    if chars.len() < 2 {
        return false;
    }
    if needs(chars[1]) {
        return true;
    }
    could_be_var = could_be_var && chars[1].eq_ignore_ascii_case(&'a');

    if chars.len() < 3 {
        return false;
    }
    if needs(chars[2]) {
        return true;
    }
    could_be_var = could_be_var && chars[2].eq_ignore_ascii_case(&'r');

    if chars.len() < 4 {
        return false;
    }
    if could_be_var && chars[3] == '(' {
        return true;
    }
    chars[3..].iter().any(|&c| needs(c))
}
