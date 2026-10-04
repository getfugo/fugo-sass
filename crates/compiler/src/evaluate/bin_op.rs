use std::cmp::Ordering;

use codemap::Span;

use crate::{
    Options,
    common::{BinaryOp, QuoteKind},
    error::SassResult,
    unit::Unit,
    value::{SassNumber, Value},
};

mod additive;

pub(crate) use additive::{add, sub};

pub(crate) fn mul(left: Value, right: Value, _: &Options, span: Span) -> SassResult<Value> {
    Ok(match left {
        Value::Dimension(SassNumber {
            num,
            unit,
            as_slash: _,
        }) => match right {
            Value::Dimension(SassNumber {
                num: num2,
                unit: unit2,
                as_slash: _,
            }) => {
                if unit2 == Unit::None {
                    return Ok(Value::Dimension(SassNumber {
                        num: num * num2,
                        unit,
                        as_slash: None,
                    }));
                }

                let n = SassNumber {
                    num,
                    unit,
                    as_slash: None,
                } * SassNumber {
                    num: num2,
                    unit: unit2,
                    as_slash: None,
                };

                Value::Dimension(n)
            }
            _ => {
                return Err((
                    format!(
                        "Undefined operation \"{}{} * {}\".",
                        num.inspect(),
                        unit,
                        right.inspect(span)?
                    ),
                    span,
                )
                    .into());
            }
        },
        _ => {
            return Err((
                format!(
                    "Undefined operation \"{} * {}\".",
                    left.inspect(span)?,
                    right.inspect(span)?
                ),
                span,
            )
                .into());
        }
    })
}

pub(crate) fn cmp(
    left: &Value,
    right: &Value,
    _: &Options,
    span: Span,
    op: BinaryOp,
) -> SassResult<Value> {
    let ordering = match left.cmp(right, span, op)? {
        Some(ord) => ord,
        None => return Ok(Value::False),
    };

    Ok(match op {
        BinaryOp::GreaterThan => match ordering {
            Ordering::Greater => Value::True,
            Ordering::Less | Ordering::Equal => Value::False,
        },
        BinaryOp::GreaterThanEqual => match ordering {
            Ordering::Greater | Ordering::Equal => Value::True,
            Ordering::Less => Value::False,
        },
        BinaryOp::LessThan => match ordering {
            Ordering::Less => Value::True,
            Ordering::Greater | Ordering::Equal => Value::False,
        },
        BinaryOp::LessThanEqual => match ordering {
            Ordering::Less | Ordering::Equal => Value::True,
            Ordering::Greater => Value::False,
        },
        _ => unreachable!(),
    })
}

pub(crate) fn single_eq(
    left: &Value,
    right: &Value,
    options: &Options,
    span: Span,
) -> SassResult<Value> {
    Ok(Value::String(
        format!(
            "{}={}",
            left.to_css_string(span, options.is_compressed())?,
            right.to_css_string(span, options.is_compressed())?
        ),
        QuoteKind::None,
    ))
}

pub(crate) fn div(left: Value, right: Value, options: &Options, span: Span) -> SassResult<Value> {
    Ok(match (left, right) {
        (Value::Dimension(num1), Value::Dimension(num2)) => {
            if num2.unit == Unit::None {
                return Ok(Value::Dimension(SassNumber {
                    num: num1.num / num2.num,
                    unit: num1.unit,
                    as_slash: None,
                }));
            }

            let n = SassNumber {
                num: num1.num,
                unit: num1.unit,
                as_slash: None,
            } / SassNumber {
                num: num2.num,
                unit: num2.unit,
                as_slash: None,
            };

            Value::Dimension(n)
        }
        (
            left @ Value::Color(..),
            right @ (Value::Dimension(SassNumber { .. }) | Value::Color(..)),
        ) => {
            return Err((
                format!(
                    "Undefined operation \"{} / {}\".",
                    left.inspect(span)?,
                    right.inspect(span)?
                ),
                span,
            )
                .into());
        }
        (left, right) => Value::String(
            format!(
                "{}/{}",
                left.to_css_string(span, options.is_compressed())?,
                right.to_css_string(span, options.is_compressed())?
            ),
            QuoteKind::None,
        ),
    })
}

pub(crate) fn rem(left: Value, right: Value, _: &Options, span: Span) -> SassResult<Value> {
    Ok(match (left, right) {
        (Value::Dimension(num1), Value::Dimension(num2)) => {
            if !num1.unit.comparable(&num2.unit) {
                return Err((
                    format!("Incompatible units {} and {}.", num1.unit, num2.unit),
                    span,
                )
                    .into());
            }

            let new_num = num1.num % num2.num.convert(&num2.unit, &num1.unit);
            let new_unit = if num1.unit == num2.unit {
                num1.unit
            } else if num1.unit == Unit::None {
                num2.unit
            } else {
                num1.unit
            };
            Value::Dimension(SassNumber {
                num: new_num,
                unit: new_unit,
                as_slash: None,
            })
        }
        (left, right) => {
            return Err((
                format!(
                    "Undefined operation \"{} % {}\".",
                    left.inspect(span)?,
                    right.inspect(span)?
                ),
                span,
            )
                .into());
        }
    })
}
