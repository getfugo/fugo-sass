//! Comparisons and unary operations.

use super::*;

impl Value {
    pub fn cmp(&self, other: &Self, span: Span, op: BinaryOp) -> SassResult<Option<Ordering>> {
        Ok(match self {
            Value::Dimension(SassNumber { num, unit, .. }) => match &other {
                Value::Dimension(SassNumber {
                    num: num2,
                    unit: unit2,
                    ..
                }) => {
                    if !unit.comparable(unit2) {
                        return Err(
                            (format!("Incompatible units {} and {}.", unit2, unit), span).into(),
                        );
                    }
                    if unit == unit2 || unit == &Unit::None || unit2 == &Unit::None {
                        num.partial_cmp(num2)
                    } else {
                        num.partial_cmp(&num2.convert(unit2, unit))
                    }
                }
                _ => {
                    return Err((
                        format!(
                            "Undefined operation \"{} {} {}\".",
                            self.inspect(span)?,
                            op,
                            other.inspect(span)?
                        ),
                        span,
                    )
                        .into());
                }
            },
            _ => {
                return Err((
                    format!(
                        "Undefined operation \"{} {} {}\".",
                        self.inspect(span)?,
                        op,
                        other.inspect(span)?
                    ),
                    span,
                )
                    .into());
            }
        })
    }

    pub fn not_equals(&self, other: &Self) -> bool {
        match self {
            Value::String(s1, ..) => match other {
                Value::String(s2, ..) => s1 != s2,
                _ => true,
            },
            Value::Dimension(SassNumber {
                num: n,
                unit,
                as_slash: _,
            }) if !n.is_nan() => match other {
                Value::Dimension(SassNumber {
                    num: n2,
                    unit: unit2,
                    as_slash: _,
                }) if !n2.is_nan() => {
                    if !unit.comparable(unit2) {
                        true
                    } else if unit == unit2 {
                        n != n2
                    } else if unit == &Unit::None || unit2 == &Unit::None {
                        true
                    } else {
                        n != &n2.convert(unit2, unit)
                    }
                }
                _ => true,
            },
            Value::List(list1, sep1, brackets1) => match other {
                Value::List(list2, sep2, brackets2) => {
                    if sep1 != sep2 || brackets1 != brackets2 || list1.len() != list2.len() {
                        true
                    } else {
                        for (a, b) in list1.iter().zip(list2) {
                            if a.not_equals(b) {
                                return true;
                            }
                        }
                        false
                    }
                }
                _ => true,
            },
            s => s != other,
        }
    }

    pub fn unary_plus(self, visitor: &mut Visitor, span: Span) -> SassResult<Self> {
        Ok(match self {
            Self::Dimension(SassNumber { .. }) => self,
            Self::Calculation(..) => {
                return Err((
                    format!("Undefined operation \"+{}\".", self.inspect(span)?),
                    span,
                )
                    .into());
            }
            _ => Self::String(
                format!(
                    "+{}",
                    self.to_css_string(span, visitor.options.is_compressed())?
                ),
                QuoteKind::None,
            ),
        })
    }

    pub fn unary_neg(self, visitor: &mut Visitor, span: Span) -> SassResult<Self> {
        Ok(match self {
            Self::Calculation(..) => {
                return Err((
                    format!("Undefined operation \"-{}\".", self.inspect(span)?),
                    span,
                )
                    .into());
            }
            Self::Dimension(SassNumber {
                num,
                unit,
                as_slash,
            }) => Self::Dimension(SassNumber {
                num: -num,
                unit,
                as_slash,
            }),
            _ => Self::String(
                format!(
                    "-{}",
                    self.to_css_string(span, visitor.options.is_compressed())?
                ),
                QuoteKind::None,
            ),
        })
    }

    pub fn unary_div(self, visitor: &mut Visitor, span: Span) -> SassResult<Self> {
        Ok(Self::String(
            format!(
                "/{}",
                self.to_css_string(span, visitor.options.is_compressed())?
            ),
            QuoteKind::None,
        ))
    }

    pub fn unary_not(self) -> Self {
        match self {
            Self::False | Self::Null => Self::True,
            _ => Self::False,
        }
    }
}
