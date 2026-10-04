//! The CSS `if()` function.

use super::*;

/// The value of a CSS `if()` condition: known at compile time, or CSS for the browser.
pub(super) enum IfConditionResult {
    Bool(bool),
    Css(String),
}

impl<'a> Visitor<'a> {
    /// Evaluates a CSS `if()`: the first branch whose condition is true at compile time, or an
    /// `if()` of the branches whose conditions are only known in the browser, or null.
    pub(super) fn visit_if_expr(&mut self, if_expr: &IfExpr) -> SassResult<Value> {
        let mut results: Option<Vec<(String, Value)>> = None;
        for (condition, expression) in &if_expr.branches {
            let result = match condition {
                Some(condition) => self.visit_if_condition(condition)?,
                None => IfConditionResult::Bool(true),
            };
            match (result, &mut results) {
                (IfConditionResult::Css(condition), results) => {
                    let value = self.visit_expr(expression.node.clone())?;
                    results
                        .get_or_insert_with(Vec::new)
                        .push((condition, value));
                }
                (IfConditionResult::Bool(true), Some(results)) => {
                    let value = self.visit_expr(expression.node.clone())?;
                    results.push(("else".to_owned(), value));
                }
                (IfConditionResult::Bool(true), None) => {
                    return self.visit_expr(expression.node.clone());
                }
                (IfConditionResult::Bool(false), _) => {}
            }
        }

        let Some(results) = results else {
            return Ok(Value::Null);
        };
        // dart-sass writes the values as expanded CSS, whatever the output style.
        let branches = results
            .into_iter()
            .map(|(condition, value)| {
                Ok(format!(
                    "{condition}: {}",
                    value.to_css_string(if_expr.span, false)?
                ))
            })
            .collect::<SassResult<Vec<_>>>()?;
        Ok(Value::String(
            format!("if({})", branches.join("; ")),
            QuoteKind::None,
        ))
    }

    pub(super) fn visit_if_condition(
        &mut self,
        condition: &IfCondition,
    ) -> SassResult<IfConditionResult> {
        Ok(match condition {
            IfCondition::Parenthesized(expression, _) => {
                match self.visit_if_condition(expression)? {
                    IfConditionResult::Css(css) => IfConditionResult::Css(format!("({css})")),
                    result @ IfConditionResult::Bool(..) => result,
                }
            }
            IfCondition::Negation(expression, _) => match self.visit_if_condition(expression)? {
                IfConditionResult::Css(css) => IfConditionResult::Css(format!("not {css}")),
                IfConditionResult::Bool(result) => IfConditionResult::Bool(!result),
            },
            IfCondition::Operation(expressions, op) => {
                let mut values: Option<Vec<(&IfCondition, String)>> = None;
                for expression in expressions {
                    match self.visit_if_condition(expression)? {
                        IfConditionResult::Css(css) => {
                            values.get_or_insert_with(Vec::new).push((expression, css));
                        }
                        IfConditionResult::Bool(false) if *op == IfConditionOp::And => {
                            return Ok(IfConditionResult::Bool(false));
                        }
                        IfConditionResult::Bool(true) if *op == IfConditionOp::Or => {
                            return Ok(IfConditionResult::Bool(true));
                        }
                        IfConditionResult::Bool(..) => {}
                    }
                }
                match values {
                    None => IfConditionResult::Bool(*op == IfConditionOp::And),
                    // A lone parenthesized condition left of the operation loses its parentheses,
                    // since both are an `<if-group>`.
                    Some(values)
                        if values.len() == 1
                            && matches!(values[0].0, IfCondition::Parenthesized(..)) =>
                    {
                        let css = &values[0].1;
                        IfConditionResult::Css(css[1..css.len() - 1].to_owned())
                    }
                    Some(values) => IfConditionResult::Css(
                        values
                            .into_iter()
                            .map(|(_, css)| css)
                            .collect::<Vec<_>>()
                            .join(&format!(" {} ", op.as_str())),
                    ),
                }
            }
            IfCondition::Function {
                name, arguments, ..
            } => IfConditionResult::Css(format!(
                "{}({})",
                self.perform_interpolation(name.clone(), false)?,
                self.perform_interpolation(arguments.clone(), false)?
            )),
            IfCondition::Sass(expression, _) => {
                IfConditionResult::Bool(self.visit_expr(expression.node.clone())?.is_truthy())
            }
            IfCondition::Raw(text, _) => {
                IfConditionResult::Css(self.perform_interpolation(text.clone(), false)?)
            }
        })
    }
}
