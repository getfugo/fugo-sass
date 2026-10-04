use std::{iter::Iterator, marker::PhantomData, sync::Arc};

use codemap::Spanned;

use crate::{
    ContextFlags, Token,
    ast::*,
    color::{Color, ColorFormat, NAMED_COLORS},
    common::{BinaryOp, Brackets, Identifier, ListSeparator, QuoteKind, UnaryOp, unvendor},
    error::SassResult,
    unit::Unit,
    utils::as_hex,
    value::Number,
};

use super::StylesheetParser;

use super::{
    CssIfParser, ExpressionParser, IdentifierParser, ImportParser, InterpolationParser,
    ModuleParser, RawValueParser,
};

mod expression;
mod identifiers;
mod literals;
mod single;

pub(crate) type Predicate<'c, P> = &'c dyn Fn(&mut P) -> SassResult<bool>;

fn is_hex_color(interpolation: &Interpolation) -> bool {
    if let Some(plain) = interpolation.as_plain() {
        if ![3, 4, 6, 8].contains(&plain.len()) {
            return false;
        }

        return plain.chars().all(|c| c.is_ascii_hexdigit());
    }

    false
}

pub(crate) struct ValueParser<'a, 'c, P: StylesheetParser<'a>> {
    comma_expressions: Option<Vec<Spanned<AstExpr>>>,
    space_expressions: Option<Vec<Spanned<AstExpr>>>,
    binary_operators: Option<Vec<BinaryOp>>,
    /// Per operator of `binary_operators`: whether whitespace (or a comment) is on both its sides.
    operator_whitespace: Vec<bool>,
    operands: Option<Vec<Spanned<AstExpr>>>,
    allow_slash: bool,
    single_expression: Option<Spanned<AstExpr>>,
    start: usize,
    inside_bracketed_list: bool,
    single_equals: bool,
    /// Whether newlines are whitespace in the indented syntax (dart-sass's `consumeNewlines`).
    consume_newlines: bool,
    parse_until: Option<Predicate<'c, P>>,
    _a: PhantomData<&'a ()>,
}

impl<'a, 'c, P: StylesheetParser<'a>> ValueParser<'a, 'c, P> {
    pub fn parse_expression(
        parser: &mut P,
        parse_until: Option<Predicate<'c, P>>,
        inside_bracketed_list: bool,
        single_equals: bool,
        consume_newlines: bool,
    ) -> SassResult<Spanned<AstExpr>> {
        let start = parser.toks().cursor();
        let mut value_parser = Self::new(
            parser,
            parse_until,
            inside_bracketed_list,
            single_equals,
            consume_newlines,
        );

        if let Some(parse_until) = value_parser.parse_until
            && parse_until(parser)?
        {
            return Err(("Expected expression.", parser.toks().current_span()).into());
        }

        if value_parser.inside_bracketed_list {
            let bracket_start = parser.toks().cursor();

            parser.expect_char('[')?;
            parser.whitespace_with_newlines()?;

            if parser.scan_char(']') {
                return Ok(AstExpr::List(ListExpr {
                    elems: Vec::new(),
                    separator: ListSeparator::Undecided,
                    brackets: Brackets::Bracketed,
                })
                .span(parser.toks_mut().span_from(bracket_start)));
            }
        };

        value_parser.start = parser.toks().cursor();

        value_parser.single_expression = Some(value_parser.parse_single_expression(parser)?);

        let mut value = value_parser.parse_value(parser)?;
        value.span = parser.toks_mut().span_from(start);

        Ok(value)
    }

    pub fn new(
        parser: &mut P,
        parse_until: Option<Predicate<'c, P>>,
        inside_bracketed_list: bool,
        single_equals: bool,
        consume_newlines: bool,
    ) -> Self {
        Self {
            comma_expressions: None,
            space_expressions: None,
            binary_operators: None,
            operator_whitespace: Vec::new(),
            operands: None,
            allow_slash: true,
            start: parser.toks().cursor(),
            single_expression: None,
            parse_until,
            inside_bracketed_list,
            single_equals,
            consume_newlines,
            _a: PhantomData,
        }
    }

    /// Parse a value from a stream of tokens
    ///
    /// This function will cease parsing if the predicate returns true.
    fn whitespace(&self, parser: &mut P) -> SassResult<()> {
        if self.consume_newlines || self.inside_bracketed_list {
            parser.whitespace_with_newlines()
        } else {
            parser.whitespace()
        }
    }

    fn reset_state(&mut self, parser: &mut P) -> SassResult<()> {
        self.comma_expressions = None;
        self.space_expressions = None;
        self.binary_operators = None;
        self.operator_whitespace.clear();
        self.operands = None;
        parser.toks_mut().set_cursor(self.start);
        self.allow_slash = true;
        self.single_expression = Some(self.parse_single_expression(parser)?);

        Ok(())
    }
}
