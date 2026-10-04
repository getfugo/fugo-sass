//! Calculations: `calc()`, `min()`, `max()`, `clamp()` and the CSS Values 4 functions
//! (`round()`, `mod()`, `sin()`, …), ported from dart-sass's `SassCalculation`
//! (`lib/src/value/calculation.dart`, dart-sass 1.105.1).

use core::fmt;
use std::{f64::consts, iter::Iterator};

use codemap::Span;

use crate::{
    Options,
    common::BinaryOp,
    error::SassResult,
    serializer::inspect_number,
    unit::Unit,
    value::{Number, SassNumber, Value, conversion_factor, fuzzy_less_than, sign_including_zero},
};

mod functions;
mod stepped;

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
                if !compatible
                    && let Some(name) = in_legacy_sass_function
                    && l.is_comparable_to(r)
                {
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

            if let CalculationArg::Number(n) = &mut right
                && fuzzy_less_than(n.num.0, 0.0)
            {
                n.num = -n.num;
                op = if op == BinaryOp::Plus {
                    BinaryOp::Minus
                } else {
                    BinaryOp::Plus
                };
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
            if let CalculationArg::Number(n) = arg
                && n.unit.is_complex()
            {
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
