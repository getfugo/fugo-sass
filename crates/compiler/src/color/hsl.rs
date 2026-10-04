//! HSL: the hue, saturation and lightness of colors, and the functions that adjust them.

use super::*;

/// HSLA color functions
/// Algorithms adapted from <http://www.niwa.nu/2013/05/math-behind-colorspace-conversions-rgb-hsl/>
impl Color {
    /// Calculate hue from RGBA values
    pub fn hue(&self) -> Number {
        if let Some(h) = &self.hsla {
            return h.hue();
        }

        let red = self.red() / Number(255.0);
        let green = self.green() / Number(255.0);
        let blue = self.blue() / Number(255.0);

        let min = red.min(green.min(blue));
        let max = red.max(green.max(blue));

        let delta = max - min;

        let hue = if min == max {
            Number::zero()
        } else if max == red {
            Number(60.0) * (green - blue) / delta
        } else if max == green {
            Number(120.0) + Number(60.0) * (blue - red) / delta
        } else {
            Number(240.0) + Number(60.0) * (red - green) / delta
        };

        hue % Number(360.0)
    }

    /// Calculate saturation from RGBA values
    pub fn saturation(&self) -> Number {
        if let Some(h) = &self.hsla {
            return h.saturation() * Number(100.0);
        }

        let red: Number = self.red() / Number(255.0);
        let green = self.green() / Number(255.0);
        let blue = self.blue() / Number(255.0);

        let min = red.min(green.min(blue));
        let max = red.max(green.max(blue));

        if min == max {
            return Number::zero();
        }

        let delta = max - min;

        let sum = max + min;

        let s = delta
            / if sum > Number::one() {
                Number(2.0) - sum
            } else {
                sum
            };

        s * Number(100.0)
    }

    /// Calculate luminance from RGBA values
    pub fn lightness(&self) -> Number {
        if let Some(h) = &self.hsla {
            return h.luminance() * Number(100.0);
        }

        let red: Number = self.red() / Number(255.0);
        let green = self.green() / Number(255.0);
        let blue = self.blue() / Number(255.0);
        let min = red.min(green.min(blue));
        let max = red.max(green.max(blue));
        (((min + max) / Number(2.0)) * Number(100.0)).round()
    }

    pub fn as_hsla(&self) -> (Number, Number, Number, Number) {
        if let Some(h) = &self.hsla {
            return (h.hue(), h.saturation(), h.luminance(), self.alpha());
        }

        let red = self.red() / Number(255.0);
        let green = self.green() / Number(255.0);
        let blue = self.blue() / Number(255.0);
        let min = red.min(green.min(blue));
        let max = red.max(green.max(blue));

        let lightness = (min + max) / Number(2.0);

        let saturation = if min == max {
            Number::zero()
        } else {
            let d = max - min;
            let mm = max + min;
            d / if mm > Number::one() {
                Number(2.0) - mm
            } else {
                mm
            }
        };

        let mut hue = if min == max {
            Number::zero()
        } else if blue == max {
            Number(4.0) + (red - green) / (max - min)
        } else if green == max {
            Number(2.0) + (blue - red) / (max - min)
        } else {
            (green - blue) / (max - min)
        };

        if hue.is_negative() {
            hue += Number(360.0);
        }

        hue *= Number(60.0);

        (hue % Number(360.0), saturation, lightness, self.alpha())
    }

    pub fn adjust_hue(&self, degrees: Number) -> Self {
        let (hue, saturation, luminance, alpha) = self.as_hsla();
        Color::from_hsla(hue + degrees, saturation, luminance, alpha)
    }

    pub fn lighten(&self, amount: Number) -> Self {
        let (hue, saturation, luminance, alpha) = self.as_hsla();
        Color::from_hsla(hue, saturation, luminance + amount, alpha)
    }

    pub fn darken(&self, amount: Number) -> Self {
        let (hue, saturation, luminance, alpha) = self.as_hsla();
        Color::from_hsla(hue, saturation, luminance - amount, alpha)
    }

    pub fn saturate(&self, amount: Number) -> Self {
        let (hue, saturation, luminance, alpha) = self.as_hsla();
        Color::from_hsla(hue, (saturation + amount).clamp(0.0, 1.0), luminance, alpha)
    }

    pub fn desaturate(&self, amount: Number) -> Self {
        let (hue, saturation, luminance, alpha) = self.as_hsla();
        Color::from_hsla(hue, (saturation - amount).clamp(0.0, 1.0), luminance, alpha)
    }

    pub fn from_hsla_fn(hue: Number, saturation: Number, luminance: Number, alpha: Number) -> Self {
        let mut color = Self::from_hsla(hue, saturation, luminance, alpha);
        color.format = ColorFormat::Hsl;
        color
    }

    /// Create RGBA representation from HSLA values
    pub fn from_hsla(hue: Number, saturation: Number, lightness: Number, alpha: Number) -> Self {
        let hue = hue % Number(360.0);
        let hsla = Hsl::new(hue, saturation.clamp(0.0, 1.0), lightness.clamp(0.0, 1.0));

        let scaled_hue = hue.0 / 360.0;
        let scaled_saturation = saturation.0.clamp(0.0, 1.0);
        let scaled_lightness = lightness.0.clamp(0.0, 1.0);

        let m2 = if scaled_lightness <= 0.5 {
            scaled_lightness * (scaled_saturation + 1.0)
        } else {
            scaled_lightness.mul_add(-scaled_saturation, scaled_lightness + scaled_saturation)
        };

        let m1 = scaled_lightness.mul_add(2.0, -m2);

        let red = fuzzy_round(Self::hue_to_rgb(m1, m2, scaled_hue + 1.0 / 3.0) * 255.0);
        let green = fuzzy_round(Self::hue_to_rgb(m1, m2, scaled_hue) * 255.0);
        let blue = fuzzy_round(Self::hue_to_rgb(m1, m2, scaled_hue - 1.0 / 3.0) * 255.0);

        Color::new_hsla(Number(red), Number(green), Number(blue), alpha, hsla)
    }

    pub(super) fn hue_to_rgb(m1: f64, m2: f64, mut hue: f64) -> f64 {
        if hue < 0.0 {
            hue += 1.0;
        }
        if hue > 1.0 {
            hue -= 1.0;
        }

        if hue < 1.0 / 6.0 {
            ((m2 - m1) * hue).mul_add(6.0, m1)
        } else if hue < 1.0 / 2.0 {
            m2
        } else if hue < 2.0 / 3.0 {
            ((m2 - m1) * (2.0 / 3.0 - hue)).mul_add(6.0, m1)
        } else {
            m1
        }
    }

    pub fn invert(&self, weight: Number) -> Self {
        if weight.is_zero() {
            return self.clone();
        }

        let red = Number(255.0) - self.red();
        let green = Number(255.0) - self.green();
        let blue = Number(255.0) - self.blue();

        let inverse = Color::new_rgba(red, green, blue, self.alpha(), ColorFormat::Infer);

        inverse.mix(self, weight)
    }

    pub fn complement(&self) -> Self {
        let (hue, saturation, luminance, alpha) = self.as_hsla();

        Color::from_hsla(hue + Number(180.0), saturation, luminance, alpha)
    }
}
