//! `+` and `-`.

use super::*;

pub(crate) fn add(left: Value, right: Value, options: &Options, span: Span) -> SassResult<Value> {
    Ok(match left {
        Value::Calculation(..) => match right {
            Value::String(s, quotes) => Value::String(
                format!(
                    "{}{}",
                    left.to_css_string(span, options.is_compressed())?,
                    s
                ),
                quotes,
            ),
            _ => {
                return Err((
                    format!(
                        "Undefined operation \"{} + {}\".",
                        left.inspect(span)?,
                        right.inspect(span)?
                    ),
                    span,
                )
                    .into());
            }
        },
        Value::Map(..) | Value::FunctionRef(..) | Value::MixinRef(..) => {
            return Err((
                format!("{} isn't a valid CSS value.", left.inspect(span)?),
                span,
            )
                .into());
        }
        Value::True | Value::False => match right {
            Value::String(s, QuoteKind::Quoted) => Value::String(
                format!(
                    "{}{}",
                    left.to_css_string(span, options.is_compressed())?,
                    s
                ),
                QuoteKind::Quoted,
            ),
            _ => Value::String(
                format!(
                    "{}{}",
                    left.to_css_string(span, options.is_compressed())?,
                    right.to_css_string(span, options.is_compressed())?
                ),
                QuoteKind::None,
            ),
        },
        Value::Null => match right {
            Value::Null => Value::Null,
            _ => Value::String(
                right.to_css_string(span, options.is_compressed())?,
                QuoteKind::None,
            ),
        },
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
                if !unit.comparable(&unit2) {
                    return Err(
                        (format!("Incompatible units {} and {}.", unit2, unit), span).into(),
                    );
                }
                if unit == unit2 {
                    Value::Dimension(SassNumber {
                        num: num + num2,
                        unit,
                        as_slash: None,
                    })
                } else if unit == Unit::None {
                    Value::Dimension(SassNumber {
                        num: num + num2,
                        unit: unit2,
                        as_slash: None,
                    })
                } else if unit2 == Unit::None {
                    Value::Dimension(SassNumber {
                        num: num + num2,
                        unit,
                        as_slash: None,
                    })
                } else {
                    Value::Dimension(SassNumber {
                        num: num + num2.convert(&unit2, &unit),
                        unit,
                        as_slash: None,
                    })
                }
            }
            Value::String(s, q) => Value::String(
                format!("{}{}{}", num.to_string(options.is_compressed()), unit, s),
                q,
            ),
            Value::Null => Value::String(
                format!("{}{}", num.to_string(options.is_compressed()), unit),
                QuoteKind::None,
            ),
            Value::True | Value::False | Value::List(..) | Value::ArgList(..) => Value::String(
                format!(
                    "{}{}{}",
                    num.to_string(options.is_compressed()),
                    unit,
                    right.to_css_string(span, options.is_compressed())?
                ),
                QuoteKind::None,
            ),
            Value::Map(..) | Value::FunctionRef(..) | Value::MixinRef(..) => {
                return Err((
                    format!("{} isn't a valid CSS value.", right.inspect(span)?),
                    span,
                )
                    .into());
            }
            Value::Color(..) | Value::Calculation(..) => {
                return Err((
                    format!(
                        "Undefined operation \"{}{} + {}\".",
                        num.inspect(),
                        unit,
                        right.inspect(span)?
                    ),
                    span,
                )
                    .into());
            }
        },
        c @ Value::Color(..) => match right {
            // todo: we really can't add to any other types?
            Value::String(..) | Value::Null | Value::List(..) => Value::String(
                format!(
                    "{}{}",
                    c.to_css_string(span, options.is_compressed())?,
                    right.to_css_string(span, options.is_compressed())?,
                ),
                QuoteKind::None,
            ),
            _ => {
                return Err((
                    format!(
                        "Undefined operation \"{} + {}\".",
                        c.inspect(span)?,
                        right.inspect(span)?
                    ),
                    span,
                )
                    .into());
            }
        },
        Value::String(text, quotes) => match right {
            Value::String(text2, ..) => Value::String(text + &text2, quotes),
            _ => Value::String(
                text + &right.to_css_string(span, options.is_compressed())?,
                quotes,
            ),
        },
        Value::List(..) | Value::ArgList(..) => match right {
            Value::String(s, q) => Value::String(
                format!(
                    "{}{}",
                    left.to_css_string(span, options.is_compressed())?,
                    s
                ),
                q,
            ),
            _ => Value::String(
                format!(
                    "{}{}",
                    left.to_css_string(span, options.is_compressed())?,
                    right.to_css_string(span, options.is_compressed())?
                ),
                QuoteKind::None,
            ),
        },
    })
}

pub(crate) fn sub(left: Value, right: Value, options: &Options, span: Span) -> SassResult<Value> {
    Ok(match left {
        Value::Calculation(..) => {
            return Err((
                format!(
                    "Undefined operation \"{} - {}\".",
                    left.inspect(span)?,
                    right.inspect(span)?
                ),
                span,
            )
                .into());
        }
        Value::Null => Value::String(
            format!("-{}", right.to_css_string(span, options.is_compressed())?),
            QuoteKind::None,
        ),
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
                if !unit.comparable(&unit2) {
                    return Err(
                        (format!("Incompatible units {} and {}.", unit2, unit), span).into(),
                    );
                }
                if unit == unit2 {
                    Value::Dimension(SassNumber {
                        num: num - num2,
                        unit,
                        as_slash: None,
                    })
                } else if unit == Unit::None {
                    Value::Dimension(SassNumber {
                        num: num - num2,
                        unit: unit2,
                        as_slash: None,
                    })
                } else if unit2 == Unit::None {
                    Value::Dimension(SassNumber {
                        num: num - num2,
                        unit,
                        as_slash: None,
                    })
                } else {
                    Value::Dimension(SassNumber {
                        num: num - num2.convert(&unit2, &unit),
                        unit,
                        as_slash: None,
                    })
                }
            }
            Value::List(..)
            | Value::String(..)
            | Value::True
            | Value::False
            | Value::ArgList(..) => Value::String(
                format!(
                    "{}{}-{}",
                    num.to_string(options.is_compressed()),
                    unit,
                    right.to_css_string(span, options.is_compressed())?
                ),
                QuoteKind::None,
            ),
            Value::Map(..) | Value::FunctionRef(..) | Value::MixinRef(..) => {
                return Err((
                    format!("{} isn't a valid CSS value.", right.inspect(span)?),
                    span,
                )
                    .into());
            }
            Value::Color(..) | Value::Calculation(..) => {
                return Err((
                    format!(
                        "Undefined operation \"{}{} - {}\".",
                        num.inspect(),
                        unit,
                        right.inspect(span)?
                    ),
                    span,
                )
                    .into());
            }
            Value::Null => Value::String(
                format!("{}{}-", num.to_string(options.is_compressed()), unit),
                QuoteKind::None,
            ),
        },
        c @ Value::Color(..) => match right {
            Value::Dimension(SassNumber { .. }) | Value::Color(..) => {
                return Err((
                    format!(
                        "Undefined operation \"{} - {}\".",
                        c.inspect(span)?,
                        right.inspect(span)?
                    ),
                    span,
                )
                    .into());
            }
            _ => Value::String(
                format!(
                    "{}-{}",
                    c.to_css_string(span, options.is_compressed())?,
                    right.to_css_string(span, options.is_compressed())?
                ),
                QuoteKind::None,
            ),
        },
        Value::String(..) => Value::String(
            format!(
                "{}-{}",
                left.to_css_string(span, options.is_compressed())?,
                right.to_css_string(span, options.is_compressed())?
            ),
            QuoteKind::None,
        ),
        // todo: can be greatly simplified
        _ => match right {
            Value::String(s, q) => Value::String(
                format!(
                    "{}-{}{}{}",
                    left.to_css_string(span, options.is_compressed())?,
                    q,
                    s,
                    q
                ),
                QuoteKind::None,
            ),
            Value::Null => Value::String(
                format!("{}-", left.to_css_string(span, options.is_compressed())?),
                QuoteKind::None,
            ),
            _ => Value::String(
                format!(
                    "{}-{}",
                    left.to_css_string(span, options.is_compressed())?,
                    right.to_css_string(span, options.is_compressed())?
                ),
                QuoteKind::None,
            ),
        },
    })
}
