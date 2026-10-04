use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    ffi::OsStr,
    fmt,
    iter::FromIterator,
    mem,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use codemap::{CodeMap, Span, Spanned};
use indexmap::{IndexMap, IndexSet};

use crate::{
    ContextFlags, InputSyntax, Options,
    ast::*,
    builtin::{
        GLOBAL_FUNCTIONS,
        meta::if_arguments,
        modules::{
            Module, declare_module_color, declare_module_list, declare_module_map,
            declare_module_math, declare_module_meta, declare_module_selector,
            declare_module_string,
        },
    },
    common::{BinaryOp, Brackets, Identifier, ListSeparator, QuoteKind, UnaryOp, unvendor},
    error::{SassError, SassResult},
    interner::InternedString,
    lexer::Lexer,
    parse::{
        AtRootQueryParser, CssParser, KeyframesSelectorParser, SassParser, ScssParser,
        StylesheetParser,
    },
    selector::{
        ComplexSelectorComponent, ExtendRule, ExtendedSelector, Extension, ExtensionStore,
        SelectorList, SelectorParser,
    },
    serializer::serialize_calculation_arg,
    utils::{to_sentence, trim_ascii},
    value::{
        ArgList, CalculationArg, CalculationName, Number, SassCalculation, SassFunction, SassMap,
        SassNumber, UserDefinedFunction, Value,
    },
};

use super::{
    bin_op::{add, cmp, div, mul, rem, single_eq, sub},
    css_tree::{CssTree, CssTreeIdx},
    env::Environment,
};

mod at_rules;
mod calculations;
mod callables;
mod css_if;
mod expressions;
mod extend;
mod imports;
mod interpolation;
mod modules;
mod statements;
mod style_rules;

pub(crate) use callables::CallableContentBlock;

/// Evaluation context of the current execution
#[derive(Debug)]
pub struct Visitor<'a> {
    pub(crate) declaration_name: Option<String>,
    pub(crate) flags: ContextFlags,
    pub(crate) env: Environment,
    pub(crate) style_rule_ignoring_at_root: Option<ExtendedSelector>,
    /// Whether the style rule `style_rule_ignoring_at_root` comes from plain CSS, whose nested
    /// rules are written as CSS nesting (dart-sass's `CssStyleRule.fromPlainCss`).
    style_rule_from_plain_css: bool,
    /// The node of `style_rule_ignoring_at_root` in the CSS tree.
    style_rule_idx: Option<CssTreeIdx>,
    // avoid emitting duplicate warnings for the same span
    pub(crate) warnings_emitted: HashSet<Span>,
    pub(crate) media_queries: Option<Vec<MediaQuery>>,
    pub(crate) media_query_sources: Option<IndexSet<MediaQuery>>,
    pub(crate) extender: ExtensionStore,

    /// The complete file path of the current file being visited. Imports are
    /// resolved relative to this path
    pub current_import_path: PathBuf,
    pub(crate) is_plain_css: bool,
    pub(crate) modules: BTreeMap<PathBuf, Arc<RefCell<Module>>>,
    pub(crate) active_modules: BTreeSet<PathBuf>,
    css_tree: CssTree,
    parent: Option<CssTreeIdx>,
    configuration: Rc<RefCell<Configuration>>,
    import_nodes: Vec<CssStmt>,
    /// How many root statements the modules executed by the current one wrote, to tell whether a
    /// module wrote CSS itself.
    nested_module_css: usize,
    pub options: &'a Options<'a>,
    pub(crate) map: &'a mut CodeMap,
    // todo: remove
    empty_span: Span,
    import_cache: BTreeMap<PathBuf, StyleSheet>,
    /// As a simple heuristic, we don't cache the results of an import unless it
    /// has been seen in the past. In the majority of cases, files are imported
    /// at most once.
    files_seen: BTreeSet<PathBuf>,
}

impl<'a> Visitor<'a> {
    pub fn new(
        path: &Path,
        options: &'a Options<'a>,
        map: &'a mut CodeMap,
        empty_span: Span,
    ) -> Self {
        let mut flags = ContextFlags::empty();
        flags.set(ContextFlags::IN_SEMI_GLOBAL_SCOPE, true);

        let extender = ExtensionStore::new(empty_span);

        let current_import_path = path.to_path_buf();

        Self {
            declaration_name: None,
            style_rule_ignoring_at_root: None,
            style_rule_from_plain_css: false,
            style_rule_idx: None,
            flags,
            warnings_emitted: HashSet::new(),
            media_queries: None,
            media_query_sources: None,
            env: Environment::new(),
            extender,
            css_tree: CssTree::new(),
            parent: None,
            nested_module_css: 0,
            current_import_path,
            configuration: Rc::new(RefCell::new(Configuration::empty())),
            is_plain_css: false,
            import_nodes: Vec::new(),
            modules: BTreeMap::new(),
            active_modules: BTreeSet::new(),
            options,
            empty_span,
            map,
            import_cache: BTreeMap::new(),
            files_seen: BTreeSet::new(),
        }
    }

    pub(crate) fn visit_stylesheet(&mut self, mut style_sheet: StyleSheet) -> SassResult<()> {
        self.active_modules.insert(style_sheet.url.clone());
        let was_in_plain_css = self.is_plain_css;
        self.is_plain_css = style_sheet.is_plain_css;
        mem::swap(&mut self.current_import_path, &mut style_sheet.url);

        for stmt in style_sheet.body {
            let result = self.visit_stmt(stmt)?;
            debug_assert!(result.is_none());
        }

        mem::swap(&mut self.current_import_path, &mut style_sheet.url);
        self.is_plain_css = was_in_plain_css;

        self.active_modules.remove(&style_sheet.url);

        Ok(())
    }

    pub(crate) fn finish(mut self) -> SassResult<Vec<CssStmt>> {
        self.extend_modules()?;

        let mut finished_tree = self.css_tree.finish();
        if self.import_nodes.is_empty() {
            Ok(finished_tree)
        } else {
            self.import_nodes.append(&mut finished_tree);
            Ok(self.import_nodes)
        }
    }

    fn visit_return_rule(&mut self, ret: AstReturn) -> SassResult<Option<Value>> {
        let val = self.visit_expr(ret.val)?;

        Ok(Some(self.without_slash(val)))
    }

    // todo: we really don't have to return Option<Value> from all of these children
    pub(crate) fn visit_stmt(&mut self, stmt: AstStmt) -> SassResult<Option<Value>> {
        match stmt {
            AstStmt::RuleSet(ruleset) => self.visit_ruleset(ruleset),
            AstStmt::Style(style) => self.visit_style(style),
            AstStmt::SilentComment(..) => Ok(None),
            AstStmt::If(if_stmt) => self.visit_if_stmt(if_stmt),
            AstStmt::For(for_stmt) => self.visit_for_stmt(for_stmt),
            AstStmt::Return(ret) => self.visit_return_rule(ret),
            AstStmt::Each(each_stmt) => self.visit_each_stmt(each_stmt),
            AstStmt::Media(media_rule) => self.visit_media_rule(media_rule),
            AstStmt::Include(include_stmt) => self.visit_include_stmt(include_stmt),
            AstStmt::While(while_stmt) => self.visit_while_stmt(&while_stmt),
            AstStmt::VariableDecl(decl) => self.visit_variable_decl(decl),
            AstStmt::LoudComment(comment) => self.visit_loud_comment(comment),
            AstStmt::ImportRule(import_rule) => self.visit_import_rule(import_rule),
            AstStmt::FunctionDecl(func) => {
                self.visit_function_decl(func);
                Ok(None)
            }
            AstStmt::Mixin(mixin) => {
                self.visit_mixin_decl(mixin);
                Ok(None)
            }
            AstStmt::ContentRule(content_rule) => self.visit_content_rule(content_rule),
            AstStmt::Warn(warn_rule) => {
                self.visit_warn_rule(warn_rule)?;
                Ok(None)
            }
            AstStmt::UnknownAtRule(unknown_at_rule) => self.visit_unknown_at_rule(unknown_at_rule),
            AstStmt::ErrorRule(error_rule) => Err(self.visit_error_rule(error_rule)?),
            AstStmt::Extend(extend_rule) => self.visit_extend_rule(extend_rule),
            AstStmt::AtRootRule(at_root_rule) => self.visit_at_root_rule(at_root_rule),
            AstStmt::Debug(debug_rule) => self.visit_debug_rule(debug_rule),
            AstStmt::Use(use_rule) => {
                self.visit_use_rule(use_rule)?;
                Ok(None)
            }
            AstStmt::Forward(forward_rule) => {
                self.visit_forward_rule(forward_rule)?;
                Ok(None)
            }
            AstStmt::Supports(supports_rule) => {
                self.visit_supports_rule(supports_rule)?;
                Ok(None)
            }
        }
    }

    pub(crate) fn emit_warning(&mut self, message: &str, span: Span) {
        if self.options.quiet {
            return;
        }
        let loc = self.map.look_up_span(span);
        self.options.logger.warn(loc, message);
    }

    fn with_media_queries<T>(
        &mut self,
        queries: Option<Vec<MediaQuery>>,
        sources: Option<IndexSet<MediaQuery>>,
        callback: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let old_media_queries = self.media_queries.take();
        let old_media_query_sources = self.media_query_sources.take();
        self.media_queries = queries;
        self.media_query_sources = sources;
        let result = callback(self);
        self.media_queries = old_media_queries;
        self.media_query_sources = old_media_query_sources;
        result
    }

    fn with_environment<T, F: FnOnce(&mut Self) -> T>(
        &mut self,
        env: Environment,
        callback: F,
    ) -> T {
        let mut old_env = env;
        mem::swap(&mut self.env, &mut old_env);
        let val = callback(self);
        mem::swap(&mut self.env, &mut old_env);
        val
    }

    fn add_child<F: Fn(&CssStmt) -> bool>(
        &mut self,
        node: CssStmt,
        through: Option<F>,
    ) -> CssTreeIdx {
        if self.parent.is_none() || self.parent == Some(CssTree::ROOT) {
            return self.css_tree.add_stmt(node, self.parent);
        }

        let mut parent = self.parent.unwrap();

        if let Some(through) = through {
            while parent != CssTree::ROOT && through(self.css_tree.get(parent).as_ref().unwrap()) {
                let grandparent = self.css_tree.child_to_parent.get(&parent).copied();
                debug_assert!(
                    grandparent.is_some(),
                    "through() must return false for at least one parent of $node."
                );
                parent = grandparent.unwrap();
            }

            // If the parent has a (visible) following sibling, we shouldn't add to
            // the parent. Instead, we should create a copy and add it after the
            // interstitial sibling.
            if self.css_tree.has_following_sibling(parent) {
                let grandparent = self.css_tree.child_to_parent.get(&parent).copied().unwrap();
                let parent_node = self
                    .css_tree
                    .get(parent)
                    .as_ref()
                    .map(CssStmt::copy_without_children)
                    .unwrap();
                parent = self.css_tree.add_child(parent_node, grandparent);
            }
        }

        self.css_tree.add_child(node, parent)
    }

    /// If the current parent is not the last child of its own parent, continues in a new
    /// childless copy of it (dart-sass's `_copyParentAfterSibling`), so that declarations,
    /// comments and childless at-rules after a nested rule are written after it.
    /// Whether the current style rule is written nested in another one, as plain CSS nesting
    /// (dart-sass's `_hasCssNesting`).
    fn has_css_nesting(&self) -> bool {
        if !self.style_rule_exists() {
            return false;
        }
        let mut idx = match self.style_rule_idx {
            Some(idx) => idx,
            None => return false,
        };
        while let Some(&parent) = self.css_tree.child_to_parent.get(&idx) {
            if parent == CssTree::ROOT {
                return false;
            }
            if self
                .css_tree
                .get(parent)
                .as_ref()
                .is_some_and(CssStmt::is_style_rule)
            {
                return true;
            }
            idx = parent;
        }
        false
    }

    fn copy_parent_after_sibling(&mut self) {
        let parent = match self.parent {
            Some(parent) if parent != CssTree::ROOT => parent,
            _ => return,
        };
        if !self.css_tree.has_following_sibling(parent) {
            return;
        }
        let grandparent = self.css_tree.child_to_parent[&parent];
        let copy = self
            .css_tree
            .get(parent)
            .as_ref()
            .expect("a parent is a statement")
            .copy_without_children();
        self.parent = Some(self.css_tree.add_child(copy, grandparent));
    }

    fn with_parent<F: FnOnce(&mut Self) -> SassResult<()>, FT: Fn(&CssStmt) -> bool>(
        &mut self,
        parent: CssStmt,
        // default=true
        scope_when: bool,
        callback: F,
        // todo: optional
        through: FT,
    ) -> SassResult<()> {
        let parent_idx = self.add_child(parent, Some(through));
        let old_parent = self.parent;
        self.parent = Some(parent_idx);
        let result = self.with_scope(false, scope_when, callback);
        self.parent = old_parent;
        result
    }

    fn with_scope<T, F: FnOnce(&mut Self) -> T>(
        &mut self,
        // default=false
        semi_global: bool,
        // default=true
        when: bool,
        callback: F,
    ) -> T {
        let semi_global = semi_global && self.flags.in_semi_global_scope();
        let was_in_semi_global_scope = self.flags.in_semi_global_scope();
        self.flags
            .set(ContextFlags::IN_SEMI_GLOBAL_SCOPE, semi_global);

        if !when {
            let v = callback(self);
            self.flags
                .set(ContextFlags::IN_SEMI_GLOBAL_SCOPE, was_in_semi_global_scope);

            return v;
        }

        self.env.scopes_mut().enter_new_scope();

        let v = callback(self);

        self.flags
            .set(ContextFlags::IN_SEMI_GLOBAL_SCOPE, was_in_semi_global_scope);
        self.env.scopes_mut().exit_scope();

        v
    }

    fn with_content<T>(
        &mut self,
        content: Option<Arc<CallableContentBlock>>,
        callback: impl FnOnce(&mut Self) -> T,
    ) -> T {
        let old_content = self.env.content.take();
        self.env.content = content;
        let v = callback(self);
        self.env.content = old_content;
        v
    }

    // todo: superfluous taking `expr` by value
    fn serialize(&mut self, mut expr: Value, quote: QuoteKind, span: Span) -> SassResult<String> {
        if quote == QuoteKind::None {
            expr = expr.unquote();
        }

        expr.to_css_string(span, self.options.is_compressed())
    }

    fn set_group_end(&mut self) -> Option<()> {
        if !self.style_rule_exists() {
            let children = self
                .css_tree
                .parent_to_child
                .get(&self.parent.unwrap_or(CssTree::ROOT))?;
            let child = *children.last()?;
            self.css_tree
                .get_mut(child)
                .as_mut()
                .map(CssStmt::set_group_end)?;
        }

        Some(())
    }

    fn style_rule_exists(&self) -> bool {
        !self.flags.at_root_excluding_style_rule() && self.style_rule_ignoring_at_root.is_some()
    }
}
