use std::{iter::Iterator, sync::Arc};

use codemap::{Span, Spanned};

use crate::{
    color::Color,
    common::{BinaryOp, Brackets, Identifier, ListSeparator, QuoteKind, UnaryOp},
    error::SassResult,
    unit::Unit,
    value::Number,
};

use super::{ArgumentInvocation, AstSupportsCondition, Interpolation, InterpolationPart};

/// The legacy `if($condition, $if-true, $if-false)` function, which only evaluates one of its
/// branches.
#[derive(Debug, Clone)]
pub struct Ternary(pub ArgumentInvocation);

/// A CSS `if()` expression (dart-sass 1.95), which also takes `sass()` conditions that are
/// evaluated at compile time.
#[derive(Debug, Clone)]
pub struct IfExpr {
    /// The branches, in order. A `None` condition is an `else` branch.
    ///
    /// This is never empty.
    pub branches: Vec<(Option<IfCondition>, Spanned<AstExpr>)>,
    pub span: Span,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IfConditionOp {
    And,
    Or,
}

impl IfConditionOp {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::And => "and",
            Self::Or => "or",
        }
    }
}

/// A condition of an [`IfExpr`] branch.
#[derive(Debug, Clone)]
pub enum IfCondition {
    /// `(condition)`
    Parenthesized(Box<Self>, Span),
    /// `not condition`
    Negation(Box<Self>, Span),
    /// Two or more conditions separated by the same operator.
    Operation(Vec<Self>, IfConditionOp),
    /// A plain-CSS function-style condition, such as `media(...)` or `var(...)`.
    Function {
        name: Interpolation,
        arguments: Interpolation,
        span: Span,
    },
    /// `sass(expression)`, which is true when the expression is truthy.
    Sass(Spanned<AstExpr>, Span),
    /// Raw text, possibly with interpolation: explicit interpolation, or a whole condition that
    /// uses arbitrary substitutions in place of operators.
    Raw(Interpolation, Span),
}

impl IfCondition {
    pub fn span(&self) -> Span {
        match self {
            Self::Parenthesized(_, span)
            | Self::Negation(_, span)
            | Self::Function { span, .. }
            | Self::Sass(_, span)
            | Self::Raw(_, span) => *span,
            Self::Operation(expressions, _) => expressions
                .first()
                .unwrap()
                .span()
                .merge(expressions.last().unwrap().span()),
        }
    }

    /// Whether this is an arbitrary substitution, which may be replaced with several tokens when
    /// the CSS is used.
    pub fn is_arbitrary_substitution(&self) -> bool {
        match self {
            Self::Function { name, .. } => match name.as_plain() {
                Some(name) => {
                    let lower = name.to_ascii_lowercase();
                    matches!(lower.as_str(), "if" | "var" | "attr") || lower.starts_with("--")
                }
                None => false,
            },
            Self::Raw(..) => true,
            Self::Parenthesized(..) | Self::Negation(..) | Self::Operation(..) | Self::Sass(..) => {
                false
            }
        }
    }

    /// The interpolation that produces the same text as this condition.
    ///
    /// `sass()` conditions can't be written this way, which is an error at `substitution`, the
    /// arbitrary substitution that requires it.
    pub fn to_interpolation(&self, substitution: Span) -> SassResult<Interpolation> {
        let mut buffer = Interpolation::new();
        match self {
            Self::Parenthesized(expression, _) => {
                buffer.add_char('(');
                buffer.add_interpolation(expression.to_interpolation(substitution)?);
                buffer.add_char(')');
            }
            Self::Negation(expression, _) => {
                buffer.add_string("not ".to_owned());
                buffer.add_interpolation(expression.to_interpolation(substitution)?);
            }
            Self::Operation(expressions, op) => {
                for (i, expression) in expressions.iter().enumerate() {
                    if i != 0 {
                        buffer.add_string(format!(" {} ", op.as_str()));
                    }
                    buffer.add_interpolation(expression.to_interpolation(substitution)?);
                }
            }
            Self::Function {
                name, arguments, ..
            } => {
                buffer.add_interpolation(name.clone());
                buffer.add_char('(');
                buffer.add_interpolation(arguments.clone());
                buffer.add_char(')');
            }
            Self::Sass(..) => {
                return Err((
                    "if() conditions with arbitrary substitutions may not contain sass() \
                     expressions.",
                    substitution,
                )
                    .into());
            }
            Self::Raw(text, _) => buffer.add_interpolation(text.clone()),
        }
        Ok(buffer)
    }
}

#[derive(Debug, Clone)]
pub struct ListExpr {
    pub elems: Vec<Spanned<AstExpr>>,
    pub separator: ListSeparator,
    pub brackets: Brackets,
}

#[derive(Debug, Clone)]
pub struct FunctionCallExpr {
    pub namespace: Option<Spanned<Identifier>>,
    pub name: Identifier,
    pub arguments: Arc<ArgumentInvocation>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct InterpolatedFunction {
    pub name: Interpolation,
    pub arguments: ArgumentInvocation,
    pub span: Span,
}

#[derive(Debug, Clone, Default)]
pub struct AstSassMap(pub Vec<(Spanned<AstExpr>, AstExpr)>);

#[derive(Debug, Clone)]
pub struct BinaryOpExpr {
    pub lhs: AstExpr,
    pub op: BinaryOp,
    pub rhs: AstExpr,
    pub allows_slash: bool,
    /// Whether whitespace (or a comment) is on both sides of the operator, which calculations
    /// require around `+` and `-` (dart-sass checks the text between the operands' spans).
    pub whitespace_around_operator: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum AstExpr {
    BinaryOp(Arc<BinaryOpExpr>),
    True,
    False,
    Color(Arc<Color>),
    FunctionCall(FunctionCallExpr),
    If(Arc<IfExpr>),
    LegacyIf(Arc<Ternary>),
    InterpolatedFunction(Arc<InterpolatedFunction>),
    List(ListExpr),
    Map(AstSassMap),
    Null,
    Number {
        n: Number,
        unit: Unit,
    },
    Paren(Arc<Self>),
    ParentSelector,
    String(StringExpr, Span),
    Supports(Arc<AstSupportsCondition>),
    UnaryOp(UnaryOp, Arc<Self>, Span),
    Variable {
        name: Spanned<Identifier>,
        namespace: Option<Spanned<Identifier>>,
    },
}

// todo: make quotes bool
// todo: track span inside
#[derive(Debug, Clone)]
pub struct StringExpr(pub Interpolation, pub QuoteKind);

impl StringExpr {
    fn quote_inner_text(
        text: &str,
        quote: char,
        buffer: &mut Interpolation,
        // default=false
        is_static: bool,
    ) {
        let mut chars = text.chars().peekable();
        while let Some(char) = chars.next() {
            if char == '\n' || char == '\r' {
                buffer.add_char('\\');
                buffer.add_char('a');
                if let Some(next) = chars.peek()
                    && (next.is_ascii_whitespace() || next.is_ascii_hexdigit())
                {
                    buffer.add_char(' ');
                }
            } else {
                if char == quote
                    || char == '\\'
                    || (is_static && char == '#' && chars.peek() == Some(&'{'))
                {
                    buffer.add_char('\\');
                }
                buffer.add_char(char);
            }
        }
    }

    fn best_quote<'a>(strings: impl Iterator<Item = &'a str>) -> char {
        let mut contains_double_quote = false;
        for s in strings {
            for c in s.chars() {
                if c == '\'' {
                    return '"';
                }
                if c == '"' {
                    contains_double_quote = true;
                }
            }
        }
        if contains_double_quote { '\'' } else { '"' }
    }

    pub fn as_interpolation(self, is_static: bool) -> Interpolation {
        if self.1 == QuoteKind::None {
            return self.0;
        }

        let quote = Self::best_quote(self.0.contents.iter().filter_map(|c| match c {
            InterpolationPart::Expr(..) => None,
            InterpolationPart::String(text) => Some(text.as_str()),
        }));

        let mut buffer = Interpolation::new();
        buffer.add_char(quote);

        for value in self.0.contents {
            match value {
                InterpolationPart::Expr(e) => buffer.add_expr(e),
                InterpolationPart::String(text) => {
                    Self::quote_inner_text(&text, quote, &mut buffer, is_static);
                }
            }
        }

        buffer.add_char(quote);

        buffer
    }
}

impl AstExpr {
    pub fn is_variable(&self) -> bool {
        matches!(self, Self::Variable { .. })
    }

    pub fn is_slash_operand(&self) -> bool {
        match self {
            Self::Number { .. } | Self::FunctionCall(..) => true,
            Self::BinaryOp(binop) => binop.allows_slash,
            _ => false,
        }
    }

    pub fn slash(left: Self, right: Self, span: Span) -> Self {
        Self::BinaryOp(Arc::new(BinaryOpExpr {
            lhs: left,
            op: BinaryOp::Div,
            rhs: right,
            allows_slash: true,
            whitespace_around_operator: true,
            span,
        }))
    }

    pub const fn span(self, span: Span) -> Spanned<Self> {
        Spanned { node: self, span }
    }
}

impl AstExpr {
    /// Whether this expression is valid in a calculation (dart-sass's
    /// `IsCalculationSafeVisitor`).
    pub fn is_calculation_safe(&self) -> bool {
        match self {
            Self::BinaryOp(binop) => {
                matches!(
                    binop.op,
                    BinaryOp::Mul | BinaryOp::Div | BinaryOp::Plus | BinaryOp::Minus
                ) && binop.lhs.is_calculation_safe()
                    && binop.rhs.is_calculation_safe()
            }
            Self::True | Self::False | Self::Color(..) | Self::Map(..) | Self::Null => false,
            Self::FunctionCall(..)
            | Self::If(..)
            | Self::LegacyIf(..)
            | Self::InterpolatedFunction(..) => true,
            Self::List(list) => {
                list.separator == ListSeparator::Space
                    && list.brackets == Brackets::None
                    && list.elems.len() > 1
                    && list.elems.iter().all(|e| e.node.is_calculation_safe())
            }
            Self::Number { .. } | Self::Variable { .. } => true,
            Self::Paren(inner) => inner.is_calculation_safe(),
            Self::ParentSelector | Self::Supports(..) | Self::UnaryOp(..) => false,
            Self::String(StringExpr(text, quotes), ..) => {
                if *quotes != QuoteKind::None {
                    return false;
                }
                // Exclude non-identifier constructs that are parsed as strings.
                let text = text.initial_plain();
                // `!important`, ID-style identifiers, unicode ranges, `url()`.
                !text.starts_with('!')
                    && !text.starts_with('#')
                    && text.chars().nth(1) != Some('+')
                    && text.chars().nth(3) != Some('(')
            }
        }
    }
}
