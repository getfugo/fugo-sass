//! Control flow and message rules: `@if`, `@each`, `@for`, `@while`, `@debug`, `@warn` and `@error`.

use super::*;

/// Control flow and message rules: `@if`, `@each`, `@for`, `@while`, `@debug`, `@warn` and `@error`.
pub(crate) trait ControlRuleParser<'a>: StylesheetParser<'a> {
    fn parse_debug_rule(&mut self) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let value = self.parse_expression(None, None, None)?;
        self.expect_statement_separator(Some("@debug rule"))?;

        Ok(AstStmt::Debug(AstDebugRule {
            value: value.node,
            span: value.span,
        }))
    }

    fn parse_each_rule(
        &mut self,
        child: fn(&mut Self) -> SassResult<AstStmt>,
    ) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let was_in_control_directive = self.flags().in_control_flow();
        self.flags_mut().set(ContextFlags::IN_CONTROL_FLOW, true);

        let mut variables = vec![Identifier::from(self.parse_variable_name()?)];
        self.whitespace_with_newlines()?;
        while self.scan_char(',') {
            self.whitespace_with_newlines()?;
            variables.push(Identifier::from(self.parse_variable_name()?));
            self.whitespace_with_newlines()?;
        }

        self.expect_identifier("in", false)?;
        self.whitespace_with_newlines()?;

        let list = self.parse_expression(None, None, None)?.node;

        let body = self.with_children(child)?.node;

        self.flags_mut()
            .set(ContextFlags::IN_CONTROL_FLOW, was_in_control_directive);

        Ok(AstStmt::Each(AstEach {
            variables,
            list,
            body,
        }))
    }

    fn parse_error_rule(&mut self) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let value = self.parse_expression(None, None, None)?;
        self.expect_statement_separator(Some("@error rule"))?;
        Ok(AstStmt::ErrorRule(AstErrorRule {
            value: value.node,
            span: value.span,
        }))
    }

    fn parse_for_rule(
        &mut self,
        child: fn(&mut Self) -> SassResult<AstStmt>,
    ) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let was_in_control_directive = self.flags().in_control_flow();
        self.flags_mut().set(ContextFlags::IN_CONTROL_FLOW, true);

        let var_start = self.toks().cursor();
        let variable = Spanned {
            node: Identifier::from(self.parse_variable_name()?),
            span: self.toks_mut().span_from(var_start),
        };
        self.whitespace_with_newlines()?;

        self.expect_identifier("from", false)?;
        self.whitespace_with_newlines()?;

        let exclusive: Cell<Option<bool>> = Cell::new(None);

        let from = self.parse_expression_with_newlines(Some(&|parser| {
            if !parser.looking_at_identifier() {
                return Ok(false);
            }
            Ok(if parser.scan_identifier("to", false)? {
                exclusive.set(Some(true));
                true
            } else if parser.scan_identifier("through", false)? {
                exclusive.set(Some(false));
                true
            } else {
                false
            })
        }))?;

        let is_exclusive = match exclusive.get() {
            Some(b) => b,
            None => {
                return Err((
                    "Expected \"to\" or \"through\".",
                    self.toks().current_span(),
                )
                    .into());
            }
        };

        self.whitespace_with_newlines()?;

        let to = self.parse_expression(None, None, None)?;

        let body = self.with_children(child)?.node;

        self.flags_mut()
            .set(ContextFlags::IN_CONTROL_FLOW, was_in_control_directive);

        Ok(AstStmt::For(AstFor {
            variable,
            from,
            to,
            is_exclusive,
            body,
        }))
    }

    fn parse_if_rule(
        &mut self,
        child: fn(&mut Self) -> SassResult<AstStmt>,
    ) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let if_indentation = self.current_indentation();

        let was_in_control_directive = self.flags().in_control_flow();
        self.flags_mut().set(ContextFlags::IN_CONTROL_FLOW, true);
        let condition = self.parse_expression(None, None, None)?.node;
        let body = self.parse_children(child)?;
        self.whitespace_without_comments();

        let mut clauses = vec![AstIfClause { condition, body }];

        let mut last_clause: Option<Vec<AstStmt>> = None;

        while self.scan_else(if_indentation)? {
            self.whitespace()?;
            if self.scan_identifier("if", false)? {
                self.whitespace_with_newlines()?;
                let condition = self.parse_expression(None, None, None)?.node;
                let body = self.parse_children(child)?;
                clauses.push(AstIfClause { condition, body });
            } else {
                last_clause = Some(self.parse_children(child)?);
                break;
            }
        }

        self.flags_mut()
            .set(ContextFlags::IN_CONTROL_FLOW, was_in_control_directive);
        self.whitespace_without_comments();

        Ok(AstStmt::If(AstIf {
            if_clauses: clauses,
            else_clause: last_clause,
        }))
    }

    fn parse_warn_rule(&mut self) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let value = self.parse_expression(None, None, None)?;
        self.expect_statement_separator(Some("@warn rule"))?;
        Ok(AstStmt::Warn(AstWarn {
            value: value.node,
            span: value.span,
        }))
    }

    fn parse_while_rule(
        &mut self,
        child: fn(&mut Self) -> SassResult<AstStmt>,
    ) -> SassResult<AstStmt> {
        self.whitespace_with_newlines()?;
        let was_in_control_directive = self.flags().in_control_flow();
        self.flags_mut().set(ContextFlags::IN_CONTROL_FLOW, true);

        let condition = self.parse_expression(None, None, None)?.node;

        let body = self.with_children(child)?.node;

        self.flags_mut()
            .set(ContextFlags::IN_CONTROL_FLOW, was_in_control_directive);

        Ok(AstStmt::While(AstWhile { condition, body }))
    }
}

impl<'a, T: StylesheetParser<'a>> ControlRuleParser<'a> for T {}
