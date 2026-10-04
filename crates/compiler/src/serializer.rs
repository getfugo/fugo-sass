use std::io::Write;

use codemap::{CodeMap, Span};

use crate::{
    Options,
    ast::{CssStmt, MediaQuery, Style, SupportsRule},
    color::{Color, ColorFormat, NAMED_COLORS},
    common::{BinaryOp, Brackets, ListSeparator, QuoteKind},
    error::SassResult,
    selector::{
        Combinator, ComplexSelector, ComplexSelectorComponent, CompoundSelector, Namespace, Pseudo,
        SelectorList, SimpleSelector,
    },
    unit::Unit,
    utils::hex_char_for,
    value::{
        ArgList, CalculationArg, CalculationName, SassCalculation, SassFunction, SassMap,
        SassMixin, SassNumber, Value, fuzzy_as_int, fuzzy_equals,
    },
};

mod numbers;
mod selectors;
mod statements;
mod values;

/// The serializer's output as a string. It only writes the bytes of `str`s and ASCII, so it is
/// UTF-8.
fn into_string(buffer: Vec<u8>) -> String {
    String::from_utf8(buffer).expect("the serializer writes UTF-8")
}

pub(crate) fn serialize_selector_list(
    list: &SelectorList,
    options: &Options,
    span: Span,
) -> String {
    let map = CodeMap::new();
    let mut serializer = Serializer::new(options, &map, false, span);

    serializer.write_selector_list(list);

    serializer.finish_for_expr()
}

/// A calculation argument as dart-sass's `toString()` writes it (inspect mode).
pub(crate) fn serialize_calculation_arg(
    arg: &CalculationArg,
    options: &Options,
    span: Span,
) -> SassResult<String> {
    let map = CodeMap::new();
    let mut serializer = Serializer::new(options, &map, true, span);

    serializer.write_calculation_arg(arg)?;

    Ok(serializer.finish_for_expr())
}

pub(crate) fn serialize_number(
    number: &SassNumber,
    options: &Options,
    span: Span,
) -> SassResult<String> {
    let map = CodeMap::new();
    let mut serializer = Serializer::new(options, &map, false, span);

    serializer.visit_number(number)?;

    Ok(serializer.finish_for_expr())
}

pub(crate) fn serialize_value(val: &Value, options: &Options, span: Span) -> SassResult<String> {
    let map = CodeMap::new();
    let mut serializer = Serializer::new(options, &map, false, span);

    serializer.visit_value(val, span)?;

    Ok(serializer.finish_for_expr())
}

pub(crate) fn inspect_value(val: &Value, options: &Options, span: Span) -> SassResult<String> {
    let map = CodeMap::new();
    let mut serializer = Serializer::new(options, &map, true, span);

    serializer.visit_value(val, span)?;

    Ok(serializer.finish_for_expr())
}

pub(crate) fn inspect_float(number: f64, options: &Options, span: Span) -> String {
    let map = CodeMap::new();
    let mut serializer = Serializer::new(options, &map, true, span);

    serializer.write_float(number);

    serializer.finish_for_expr()
}

pub(crate) fn inspect_map(map: &SassMap, options: &Options, span: Span) -> SassResult<String> {
    let code_map = CodeMap::new();
    let mut serializer = Serializer::new(options, &code_map, true, span);

    serializer.visit_map(map, span)?;

    Ok(serializer.finish_for_expr())
}

pub(crate) fn inspect_function_ref(
    func: &SassFunction,
    options: &Options,
    span: Span,
) -> SassResult<String> {
    let code_map = CodeMap::new();
    let mut serializer = Serializer::new(options, &code_map, true, span);

    serializer.visit_function_ref(func, span)?;

    Ok(serializer.finish_for_expr())
}

pub(crate) fn inspect_number(
    number: &SassNumber,
    options: &Options,
    span: Span,
) -> SassResult<String> {
    let map = CodeMap::new();
    let mut serializer = Serializer::new(options, &map, true, span);

    serializer.visit_number(number)?;

    Ok(serializer.finish_for_expr())
}

pub(crate) struct Serializer<'a> {
    indentation: usize,
    options: &'a Options<'a>,
    inspect: bool,
    indent_width: usize,
    // todo: use this field
    _quote: bool,
    buffer: Vec<u8>,
    map: &'a CodeMap,
}

impl<'a> Serializer<'a> {
    pub fn new(options: &'a Options<'a>, map: &'a CodeMap, inspect: bool, _span: Span) -> Self {
        Self {
            inspect,
            _quote: true,
            indentation: 0,
            indent_width: 2,
            options,
            buffer: Vec::new(),
            map,
        }
    }

    fn write_newline(&mut self) {
        if !self.options.is_compressed() {
            self.buffer.push(b'\n');
        }
    }

    fn write_comma_separator(&mut self) {
        self.buffer.push(b',');
        self.write_optional_space();
    }

    pub fn visit_group(
        &mut self,
        stmt: CssStmt,
        prev_was_group_end: bool,
        prev_requires_semicolon: bool,
    ) -> SassResult<()> {
        if prev_requires_semicolon {
            self.buffer.push(b';');
        }

        if !self.buffer.is_empty() {
            self.write_optional_newline();
        }

        if prev_was_group_end && !self.buffer.is_empty() {
            self.write_optional_newline();
        }

        self.visit_stmt(stmt)?;

        Ok(())
    }

    fn finish_for_expr(self) -> String {
        into_string(self.buffer)
    }

    pub fn finish(mut self, prev_requires_semicolon: bool) -> String {
        let is_not_ascii = self.buffer.iter().any(|&c| !c.is_ascii());

        if prev_requires_semicolon {
            self.buffer.push(b';');
        }

        if !self.buffer.is_empty() {
            self.write_optional_newline();
        }

        let mut as_string = into_string(self.buffer);

        if is_not_ascii && self.options.is_compressed() && self.options.allows_charset {
            as_string.insert(0, '\u{FEFF}');
        } else if is_not_ascii && self.options.allows_charset {
            as_string.insert_str(0, "@charset \"UTF-8\";\n");
        }

        as_string
    }

    fn write_indentation(&mut self) {
        if self.options.is_compressed() {
            return;
        }

        self.buffer.reserve(self.indentation);
        for _ in 0..self.indentation {
            self.buffer.push(b' ');
        }
    }

    fn write_optional_space(&mut self) {
        if !self.options.is_compressed() {
            self.buffer.push(b' ');
        }
    }

    fn write_optional_newline(&mut self) {
        if !self.options.is_compressed() {
            self.buffer.push(b'\n');
        }
    }
}
