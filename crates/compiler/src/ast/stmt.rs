use indexmap::{IndexMap, IndexSet};
use std::{cell::RefCell, collections::HashSet, path::PathBuf, rc::Rc, sync::Arc};

use codemap::{Span, Spanned};

use crate::{
    ast::{ArgumentDeclaration, ArgumentInvocation, AstExpr, CssStmt},
    ast::{Interpolation, MediaQuery},
    common::Identifier,
    utils::{BaseMapView, LimitedMapView, MapView, UnprefixedMapView},
    value::Value,
};

mod modules;

pub use modules::*;

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct AstSilentComment {
    pub text: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstIf {
    pub if_clauses: Vec<AstIfClause>,
    pub else_clause: Option<Vec<AstStmt>>,
}

#[derive(Debug, Clone)]
pub struct AstIfClause {
    pub condition: AstExpr,
    pub body: Vec<AstStmt>,
}

#[derive(Debug, Clone)]
pub struct AstFor {
    pub variable: Spanned<Identifier>,
    pub from: Spanned<AstExpr>,
    pub to: Spanned<AstExpr>,
    pub is_exclusive: bool,
    pub body: Vec<AstStmt>,
}

#[derive(Debug, Clone)]
pub struct AstReturn {
    pub val: AstExpr,
    #[allow(unused)]
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstRuleSet {
    pub selector: Interpolation,
    pub body: Vec<AstStmt>,
    pub selector_span: Span,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstStyle {
    pub name: Interpolation,
    pub value: Option<Spanned<AstExpr>>,
    pub body: Vec<AstStmt>,
    pub span: Span,
}

impl AstStyle {
    pub fn is_custom_property(&self) -> bool {
        self.name.initial_plain().starts_with("--")
    }
}

#[derive(Debug, Clone)]
pub struct AstEach {
    pub variables: Vec<Identifier>,
    pub list: AstExpr,
    pub body: Vec<AstStmt>,
}

#[derive(Debug, Clone)]
pub struct AstMedia {
    pub query: Interpolation,
    pub query_span: Span,
    pub body: Vec<AstStmt>,
    pub span: Span,
}

pub type CssMediaQuery = MediaQuery;

#[derive(Debug, Clone)]
pub struct AstWhile {
    pub condition: AstExpr,
    pub body: Vec<AstStmt>,
}

#[derive(Debug, Clone)]
pub struct AstVariableDecl {
    pub namespace: Option<Spanned<Identifier>>,
    pub name: Identifier,
    pub value: AstExpr,
    pub is_guarded: bool,
    pub is_global: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstFunctionDecl {
    pub name: Spanned<Identifier>,
    pub arguments: ArgumentDeclaration,
    pub body: Vec<AstStmt>,
}

#[derive(Debug, Clone)]
pub struct AstDebugRule {
    pub value: AstExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstWarn {
    pub value: AstExpr,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstErrorRule {
    pub value: AstExpr,
    pub span: Span,
}

impl PartialEq for AstFunctionDecl {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for AstFunctionDecl {}

#[derive(Debug, Clone)]
pub struct AstLoudComment {
    pub text: Interpolation,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstMixin {
    pub name: Identifier,
    pub args: ArgumentDeclaration,
    pub body: Vec<AstStmt>,
    /// Whether the mixin contains a `@content` rule.
    pub has_content: bool,
}

#[derive(Debug, Clone)]
pub struct AstContentRule {
    pub args: ArgumentInvocation,
}

#[derive(Debug, Clone)]
pub struct AstContentBlock {
    pub args: ArgumentDeclaration,
    pub body: Vec<AstStmt>,
}

#[derive(Debug, Clone)]
pub struct AstInclude {
    pub namespace: Option<Spanned<Identifier>>,
    pub name: Spanned<Identifier>,
    pub args: ArgumentInvocation,
    pub content: Option<AstContentBlock>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstUnknownAtRule {
    pub name: Interpolation,
    pub value: Option<Interpolation>,
    pub body: Option<Vec<AstStmt>>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstExtendRule {
    pub value: Interpolation,
    pub is_optional: bool,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstAtRootRule {
    pub body: Vec<AstStmt>,
    pub query: Option<Spanned<Interpolation>>,
    #[allow(unused)]
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AtRootQuery {
    pub include: bool,
    pub names: HashSet<String>,
    pub all: bool,
    pub rule: bool,
}

impl AtRootQuery {
    pub fn new(include: bool, names: HashSet<String>) -> Self {
        let all = names.contains("all");
        let rule = names.contains("rule");

        Self {
            include,
            names,
            all,
            rule,
        }
    }

    pub fn excludes_name(&self, name: &str) -> bool {
        (self.all || self.names.contains(name)) != self.include
    }

    pub fn excludes_style_rules(&self) -> bool {
        (self.all || self.rule) != self.include
    }

    pub(crate) fn excludes(&self, stmt: &CssStmt) -> bool {
        if self.all {
            return !self.include;
        }

        match stmt {
            CssStmt::RuleSet { .. } => self.excludes_style_rules(),
            CssStmt::Media(..) => self.excludes_name("media"),
            CssStmt::Supports(..) => self.excludes_name("supports"),
            CssStmt::UnknownAtRule(rule, ..) => self.excludes_name(&rule.name.to_ascii_lowercase()),
            _ => false,
        }
    }
}

impl Default for AtRootQuery {
    fn default() -> Self {
        Self {
            include: false,
            names: HashSet::new(),
            all: false,
            rule: true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AstSupportsCondition {
    Anything {
        contents: Interpolation,
    },
    Declaration {
        name: AstExpr,
        value: AstExpr,
    },
    Function {
        name: Interpolation,
        args: Interpolation,
    },
    Interpolation(AstExpr),
    Negation(Box<Self>),
    Operation {
        left: Box<Self>,
        operator: Option<String>,
        right: Box<Self>,
    },
}

#[derive(Debug, Clone)]
pub struct AstSupportsRule {
    pub condition: AstSupportsCondition,
    pub body: Vec<AstStmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum AstStmt {
    If(AstIf),
    For(AstFor),
    Return(AstReturn),
    RuleSet(AstRuleSet),
    Style(AstStyle),
    Each(AstEach),
    Media(AstMedia),
    Include(AstInclude),
    While(AstWhile),
    VariableDecl(AstVariableDecl),
    LoudComment(AstLoudComment),
    SilentComment(AstSilentComment),
    FunctionDecl(AstFunctionDecl),
    Mixin(AstMixin),
    ContentRule(AstContentRule),
    Warn(AstWarn),
    UnknownAtRule(AstUnknownAtRule),
    ErrorRule(AstErrorRule),
    Extend(AstExtendRule),
    AtRootRule(AstAtRootRule),
    Debug(AstDebugRule),
    ImportRule(AstImportRule),
    Use(AstUseRule),
    Forward(AstForwardRule),
    Supports(AstSupportsRule),
}

#[derive(Debug, Clone)]
pub struct StyleSheet {
    pub body: Vec<AstStmt>,
    pub url: PathBuf,
    pub is_plain_css: bool,
    /// Array of indices into `body`
    pub uses: Vec<usize>,
    /// Array of indices into `body`
    pub forwards: Vec<usize>,
}

impl StyleSheet {
    pub fn new(is_plain_css: bool, url: PathBuf) -> Self {
        Self {
            body: Vec::new(),
            url,
            is_plain_css,
            uses: Vec::new(),
            forwards: Vec::new(),
        }
    }
}
