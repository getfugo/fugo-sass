//! Variable declarations.

use super::*;

/// Variable declarations.
pub(crate) trait VariableParser<'a>: StylesheetParser<'a> {
    fn parse_variable_declaration_with_namespace(&mut self) -> SassResult<AstVariableDecl> {
        let start = self.toks().cursor();
        let namespace = self.parse_identifier(false, false)?;
        let namespace_span = self.toks_mut().span_from(start);
        self.expect_char('.')?;
        self.parse_variable_declaration_without_namespace(
            Some(Spanned {
                node: Identifier::namespace(&namespace),
                span: namespace_span,
            }),
            Some(start),
        )
    }

    fn parse_variable_declaration_without_namespace(
        &mut self,
        namespace: Option<Spanned<Identifier>>,
        start: Option<usize>,
    ) -> SassResult<AstVariableDecl> {
        let start = start.unwrap_or_else(|| self.toks().cursor());

        let name = self.parse_variable_name()?;

        if namespace.is_some() {
            Self::assert_public(&name, self.toks_mut().span_from(start))?;
        }

        if self.is_plain_css() {
            return Err((
                "Sass variables aren't allowed in plain CSS.",
                self.toks_mut().span_from(start),
            )
                .into());
        }

        self.whitespace_with_newlines()?;
        self.expect_char(':')?;
        self.whitespace_with_newlines()?;

        let value = self.parse_expression(None, None, None)?.node;

        let mut is_guarded = false;
        let mut is_global = false;

        while self.scan_char('!') {
            let flag_start = self.toks().cursor();
            let flag = self.parse_identifier(false, false)?;

            match flag.as_str() {
                "default" => is_guarded = true,
                "global" => {
                    if namespace.is_some() {
                        return Err((
                            "!global isn't allowed for variables in other modules.",
                            self.toks_mut().span_from(flag_start),
                        )
                            .into());
                    }

                    is_global = true;
                }
                _ => {
                    return Err(
                        ("Invalid flag name.", self.toks_mut().span_from(flag_start)).into(),
                    );
                }
            }

            self.whitespace()?;
        }

        self.expect_statement_separator(Some("variable declaration"))?;

        let declaration = AstVariableDecl {
            namespace,
            name: Identifier::from(name),
            value,
            is_guarded,
            is_global,
            span: self.toks_mut().span_from(start),
        };

        if is_global {
            // todo
            // _globalVariables.putIfAbsent(name, () => declaration)
        }

        Ok(declaration)
    }

    fn parse_variable_declaration_or_interpolation(
        &mut self,
    ) -> SassResult<VariableDeclOrInterpolation> {
        if !self.looking_at_identifier() {
            return Ok(VariableDeclOrInterpolation::Interpolation(
                self.parse_interpolated_identifier()?,
            ));
        }

        let start = self.toks().cursor();

        let ident = self.parse_identifier(false, false)?;
        if self.next_matches(".$") {
            let namespace_span = self.toks_mut().span_from(start);
            self.expect_char('.')?;
            Ok(VariableDeclOrInterpolation::VariableDecl(
                self.parse_variable_declaration_without_namespace(
                    Some(Spanned {
                        node: Identifier::namespace(&ident),
                        span: namespace_span,
                    }),
                    Some(start),
                )?,
            ))
        } else {
            let mut buffer = Interpolation::new_plain(ident);

            if self.looking_at_interpolated_identifier_body() {
                buffer.add_interpolation(self.parse_interpolated_identifier()?);
            }

            Ok(VariableDeclOrInterpolation::Interpolation(buffer))
        }
    }
}

impl<'a, T: StylesheetParser<'a>> VariableParser<'a> for T {}
