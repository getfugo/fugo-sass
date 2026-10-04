//! Interpolation.

use super::*;

impl<'a> Visitor<'a> {
    pub(super) fn interpolation_to_value(
        &mut self,
        interpolation: Interpolation,
        // default=false
        trim: bool,
        // default=false
        warn_for_color: bool,
    ) -> SassResult<String> {
        let result = self.perform_interpolation(interpolation, warn_for_color)?;

        Ok(if trim {
            trim_ascii(&result, true).to_owned()
        } else {
            result
        })
    }

    pub(super) fn perform_interpolation(
        &mut self,
        mut interpolation: Interpolation,
        // todo check to emit warning if this is true
        _warn_for_color: bool,
    ) -> SassResult<String> {
        let result = match interpolation.contents.len() {
            0 => String::new(),
            1 => match interpolation.contents.pop() {
                Some(InterpolationPart::String(s)) => s,
                Some(InterpolationPart::Expr(e)) => {
                    let span = e.span;
                    let result = self.visit_expr(e.node)?;
                    // todo: span for specific expr
                    self.serialize(result, QuoteKind::None, span)?
                }
                None => unreachable!(),
            },
            _ => interpolation
                .contents
                .into_iter()
                .map(|part| match part {
                    InterpolationPart::String(s) => Ok(s),
                    InterpolationPart::Expr(e) => {
                        let span = e.span;
                        let result = self.visit_expr(e.node)?;
                        // todo: span for specific expr
                        self.serialize(result, QuoteKind::None, span)
                    }
                })
                .collect::<SassResult<String>>()?,
        };

        Ok(result)
    }

    pub(super) fn evaluate_to_css(
        &mut self,
        expr: AstExpr,
        quote: QuoteKind,
        span: Span,
    ) -> SassResult<String> {
        let result = self.visit_expr(expr)?;
        self.serialize(result, quote, span)
    }
}
