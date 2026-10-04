//! Numbers and calculations, written as dart-sass writes them.

use super::*;

impl<'a> Serializer<'a> {
    pub(super) fn visit_calculation(&mut self, calculation: &SassCalculation) -> SassResult<()> {
        self.buffer
            .extend_from_slice(calculation.name.as_str().as_bytes());
        self.buffer.push(b'(');

        if let Some((last, slice)) = calculation.args.split_last() {
            for arg in slice {
                self.write_calculation_arg(arg)?;
                self.write_comma_separator();
            }

            self.write_calculation_arg(last)?;
        }

        self.buffer.push(b')');

        Ok(())
    }

    /// dart-sass's `_writeCalculationValue`.
    pub(super) fn write_calculation_arg(&mut self, arg: &CalculationArg) -> SassResult<()> {
        match arg {
            CalculationArg::Number(num) if !num.num.0.is_finite() => {
                let value = num.num.0;
                self.buffer.extend_from_slice(if value.is_nan() {
                    b"NaN".as_slice()
                } else if value.is_sign_negative() {
                    b"-infinity".as_slice()
                } else {
                    b"infinity".as_slice()
                });
                let (numer, denom) = num.unit.clone().numer_and_denom();
                self.write_calculation_units(&numer, &denom)?;
            }
            CalculationArg::Number(num) if num.unit.is_complex() => {
                self.write_number(num.num.0);
                let (numer, denom) = num.unit.clone().numer_and_denom();
                match numer.split_first() {
                    Some((first, rest)) => {
                        write!(&mut self.buffer, "{first}")?;
                        self.write_calculation_units(rest, &denom)?;
                    }
                    None => self.write_calculation_units(&[], &denom)?,
                }
            }
            CalculationArg::Number(num) => self.visit_number(num)?,
            CalculationArg::Calculation(calc) => {
                self.visit_calculation(calc)?;
            }
            CalculationArg::String(s) => {
                self.buffer.extend_from_slice(s.as_bytes());
            }
            CalculationArg::Operation { lhs, op, rhs } => {
                let paren_left = match &**lhs {
                    CalculationArg::Operation { op: op2, .. } => op2.precedence() < op.precedence(),
                    _ => false,
                };

                if paren_left {
                    self.buffer.push(b'(');
                }

                self.write_calculation_arg(lhs)?;

                if paren_left {
                    self.buffer.push(b')');
                }

                let operator_whitespace =
                    !self.options.is_compressed() || matches!(op, BinaryOp::Plus | BinaryOp::Minus);

                if operator_whitespace {
                    self.buffer.push(b' ');
                }

                // todo: avoid allocation with `write_binary_operator` method
                self.buffer.extend_from_slice(op.to_string().as_bytes());

                if operator_whitespace {
                    self.buffer.push(b' ');
                }

                let paren_right = match &**rhs {
                    CalculationArg::Operation { op: op2, .. } => {
                        CalculationArg::parenthesize_calculation_rhs(*op, *op2)
                    }
                    CalculationArg::Number(n) if *op == BinaryOp::Div => {
                        if n.num.0.is_finite() {
                            n.unit.is_complex()
                        } else {
                            n.unit != Unit::None
                        }
                    }
                    _ => false,
                };

                if paren_right {
                    self.buffer.push(b'(');
                }

                self.write_calculation_arg(rhs)?;

                if paren_right {
                    self.buffer.push(b')');
                }
            }
        }

        Ok(())
    }

    /// The units of a number in a calculation beyond its first numerator unit, as `* 1px` and
    /// `/ 1px` (dart-sass's `_writeCalculationUnits`).
    pub(super) fn write_calculation_units(
        &mut self,
        numer: &[Unit],
        denom: &[Unit],
    ) -> SassResult<()> {
        for unit in numer {
            self.write_optional_space();
            self.buffer.push(b'*');
            self.write_optional_space();
            write!(&mut self.buffer, "1{unit}")?;
        }
        for unit in denom {
            self.write_optional_space();
            self.buffer.push(b'/');
            self.write_optional_space();
            write!(&mut self.buffer, "1{unit}")?;
        }
        Ok(())
    }

    pub fn visit_number(&mut self, number: &SassNumber) -> SassResult<()> {
        if let Some(as_slash) = &number.as_slash {
            self.visit_number(&as_slash.0)?;
            self.buffer.push(b'/');
            self.visit_number(&as_slash.1)?;
            return Ok(());
        }

        // Infinite numbers and numbers with complex units are written as calculations.
        if !number.num.0.is_finite() || number.unit.is_complex() {
            return self.visit_calculation(&SassCalculation::unsimplified(
                CalculationName::Calc,
                vec![CalculationArg::Number(number.clone())],
            ));
        }

        self.write_number(number.num.0);
        write!(&mut self.buffer, "{}", number.unit)?;

        Ok(())
    }

    /// Writes a finite `number` without exponent notation and with at most 10 digits after the
    /// decimal point, all its digits in inspect mode (dart-sass's `_writeNumber`).
    pub(super) fn write_number(&mut self, number: f64) {
        if number == 0.0 && number.is_sign_negative() {
            self.buffer.extend_from_slice(b"-0");
            return;
        }

        // Close enough to an integer: fuzzily in inspect mode, exactly otherwise.
        let integer = if self.inspect {
            fuzzy_as_int(number).map(|i| i as f64)
        } else {
            (number.round() == number).then_some(number)
        };
        if let Some(integer) = integer {
            // `{}` writes integral floats without a fraction or an exponent.
            write!(&mut self.buffer, "{integer}").unwrap();
            return;
        }

        // The shortest representation that round-trips, as dart-sass's `toString()`.
        let text = format!("{number}");

        if self.inspect {
            self.buffer.extend_from_slice(text.as_bytes());
            return;
        }

        // At most `0.` and 10 digits: safe to write as is.
        if text.len() < 12 {
            let text = if self.options.is_compressed() && text.starts_with('0') {
                &text[1..]
            } else {
                &text
            };
            self.buffer.extend_from_slice(text.as_bytes());
            return;
        }

        self.write_rounded(&text);
    }

    /// Writes `text`, a number without exponent notation, rounded to 10 digits after the
    /// decimal point (dart-sass's `_writeRounded`).
    pub(super) fn write_rounded(&mut self, text: &str) {
        const PRECISION: usize = 10;
        let bytes = text.as_bytes();

        // Digits before and after the decimal point, starting at index 1 to leave room for
        // rounding up to a higher place than the original number has.
        let mut digits = vec![0_u8; bytes.len() + 1];
        let mut digits_index = 1;

        let mut text_index = 0;
        let negative = bytes[0] == b'-';
        if negative {
            text_index += 1;
        }
        loop {
            if text_index == bytes.len() {
                // No decimal point: nothing to round.
                self.buffer.extend_from_slice(bytes);
                return;
            }
            let byte = bytes[text_index];
            text_index += 1;
            if byte == b'.' {
                break;
            }
            digits[digits_index] = byte - b'0';
            digits_index += 1;
        }
        let first_fractional_digit = digits_index;

        let index_after_precision = text_index + PRECISION;
        if index_after_precision >= bytes.len() {
            self.buffer.extend_from_slice(bytes);
            return;
        }

        while text_index < index_after_precision {
            digits[digits_index] = bytes[text_index] - b'0';
            digits_index += 1;
            text_index += 1;
        }

        // Round the trailing digits up if necessary.
        if bytes[text_index] - b'0' >= 5 {
            loop {
                digits[digits_index - 1] += 1;
                if digits[digits_index - 1] != 10 {
                    break;
                }
                digits_index -= 1;
            }
        }

        // Digits rounded up before the decimal point become 0; trailing zeros after it go.
        while digits_index < first_fractional_digit {
            digits[digits_index] = 0;
            digits_index += 1;
        }
        while digits_index > first_fractional_digit && digits[digits_index - 1] == 0 {
            digits_index -= 1;
        }

        // Rounded to exactly zero: no minus sign, and an explicit `0` even when compressed.
        if digits_index == 2 && digits[0] == 0 && digits[1] == 0 {
            self.buffer.push(b'0');
            return;
        }

        if negative {
            self.buffer.push(b'-');
        }

        // Omit the leading 0 added for rounding, and when compressed the 0 before the point.
        let mut written_index = 0;
        if digits[0] == 0 {
            written_index += 1;
            if self.options.is_compressed() && digits[1] == 0 {
                written_index += 1;
            }
        }
        while written_index < first_fractional_digit {
            self.buffer.push(b'0' + digits[written_index]);
            written_index += 1;
        }

        if digits_index > first_fractional_digit {
            self.buffer.push(b'.');
            while written_index < digits_index {
                self.buffer.push(b'0' + digits[written_index]);
                written_index += 1;
            }
        }
    }

    pub(super) fn write_float(&mut self, float: f64) {
        if float.is_infinite() && float.is_sign_negative() {
            self.buffer.extend_from_slice(b"-Infinity");
            return;
        } else if float.is_infinite() {
            self.buffer.extend_from_slice(b"Infinity");
            return;
        }

        // todo: can optimize away intermediate buffer
        let mut buffer = String::with_capacity(3);

        if float < 0.0 {
            buffer.push('-');
        }

        let num = float.abs();

        if self.options.is_compressed() && num < 1.0 {
            buffer.push_str(
                format!("{:.10}", num)[1..]
                    .trim_end_matches('0')
                    .trim_end_matches('.'),
            );
        } else {
            buffer.push_str(
                format!("{:.10}", num)
                    .trim_end_matches('0')
                    .trim_end_matches('.'),
            );
        }

        if buffer.is_empty() || buffer == "-" || buffer == "-0" {
            buffer = "0".to_owned();
        }

        self.buffer.append(&mut buffer.into_bytes());
    }
}
