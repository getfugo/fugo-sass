//! Trigonometry: `math.cos()`, `math.sin()`, `math.tan()`, their inverses and `math.atan2()`.

use super::*;

pub(super) fn coerce_to_rad(num: f64, unit: Unit) -> f64 {
    debug_assert!(matches!(
        unit,
        Unit::None | Unit::Rad | Unit::Deg | Unit::Grad | Unit::Turn
    ));

    if unit == Unit::None {
        return num;
    }

    let factor = conversion_factor(&unit, &Unit::Rad).unwrap();

    num * factor
}

macro_rules! trig_fn {
    ($name:ident) => {
        pub(super) fn $name(mut args: ArgumentResult, _: &mut Visitor) -> SassResult<Value> {
            args.max_args(1)?;
            let number = args.get_err(0, "number")?;

            Ok(match number {
                Value::Dimension(SassNumber {
                    num,
                    unit: unit @ (Unit::None | Unit::Rad | Unit::Deg | Unit::Grad | Unit::Turn),
                    ..
                }) => {
                    Value::Dimension(SassNumber::new_unitless(coerce_to_rad(num.0, unit).$name()))
                }
                v @ Value::Dimension(..) => {
                    return Err((
                        format!(
                            "$number: Expected {} to have an angle unit (deg, grad, rad, turn).",
                            v.inspect(args.span())?
                        ),
                        args.span(),
                    )
                        .into())
                }
                v => {
                    return Err((
                        format!("$number: {} is not a number.", v.inspect(args.span())?),
                        args.span(),
                    )
                        .into())
                }
            })
        }
    };
}

trig_fn!(cos);
trig_fn!(sin);
trig_fn!(tan);

pub(super) fn acos(mut args: ArgumentResult, _: &mut Visitor) -> SassResult<Value> {
    args.max_args(1)?;

    let span = args.span();

    let number = args
        .get_err(0, "number")?
        .assert_number_with_name("number", span)?;
    number.assert_no_units("number", span)?;
    let number = number.num;

    Ok(Value::Dimension(SassNumber {
        num: if number > Number(1.0) || number < Number(-1.0) {
            Number(f64::NAN)
        } else if number.is_one() {
            Number::zero()
        } else {
            number.acos()
        },
        unit: Unit::Deg,
        as_slash: None,
    }))
}

pub(super) fn asin(mut args: ArgumentResult, _: &mut Visitor) -> SassResult<Value> {
    args.max_args(1)?;

    let span = args.span();

    let number = args
        .get_err(0, "number")?
        .assert_number_with_name("number", span)?;
    number.assert_no_units("number", span)?;
    let number = number.num;

    if number > Number(1.0) || number < Number(-1.0) {
        return Ok(Value::Dimension(SassNumber {
            num: Number(f64::NAN),
            unit: Unit::Deg,
            as_slash: None,
        }));
    } else if number.is_zero() {
        return Ok(Value::Dimension(SassNumber {
            num: Number::zero(),
            unit: Unit::Deg,
            as_slash: None,
        }));
    }

    Ok(Value::Dimension(SassNumber {
        num: number.asin(),
        unit: Unit::Deg,
        as_slash: None,
    }))
}

pub(super) fn atan(mut args: ArgumentResult, _: &mut Visitor) -> SassResult<Value> {
    args.max_args(1)?;

    let span = args.span();

    let number = args
        .get_err(0, "number")?
        .assert_number_with_name("number", span)?;
    number.assert_no_units("number", span)?;

    if number.num.is_zero() {
        return Ok(Value::Dimension(SassNumber {
            num: (Number::zero()),
            unit: Unit::Deg,
            as_slash: None,
        }));
    }

    Ok(Value::Dimension(SassNumber {
        num: number.num.atan(),
        unit: Unit::Deg,
        as_slash: None,
    }))
}

pub(super) fn atan2(mut args: ArgumentResult, _: &mut Visitor) -> SassResult<Value> {
    args.max_args(2)?;
    let (y_num, y_unit) = match args.get_err(0, "y")? {
        Value::Dimension(SassNumber {
            num: n, unit: u, ..
        }) => (n, u),
        v => {
            return Err((
                format!("$y: {} is not a number.", v.inspect(args.span())?),
                args.span(),
            )
                .into());
        }
    };

    let (x_num, x_unit) = match args.get_err(1, "x")? {
        Value::Dimension(SassNumber {
            num: n, unit: u, ..
        }) => (n, u),
        v => {
            return Err((
                format!("$x: {} is not a number.", v.inspect(args.span())?),
                args.span(),
            )
                .into());
        }
    };

    let (x_num, y_num) = if x_unit == Unit::None && y_unit == Unit::None {
        (x_num, y_num)
    } else if y_unit == Unit::None {
        return Err((
            format!(
                "$y is unitless but $x has unit {}. \
            Arguments must all have units or all be unitless.",
                x_unit
            ),
            args.span(),
        )
            .into());
    } else if x_unit == Unit::None {
        return Err((
            format!(
                "$y has unit {} but $x is unitless. \
                Arguments must all have units or all be unitless.",
                y_unit
            ),
            args.span(),
        )
            .into());
    } else if x_unit.comparable(&y_unit) {
        (x_num, y_num.convert(&y_unit, &x_unit))
    } else {
        return Err((
            format!("Incompatible units {} and {}.", y_unit, x_unit),
            args.span(),
        )
            .into());
    };

    Ok(Value::Dimension(SassNumber {
        num: Number(y_num.0.atan2(x_num.0).to_degrees()),
        unit: Unit::Deg,
        as_slash: None,
    }))
}
