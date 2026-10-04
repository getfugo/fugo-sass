//! Style rules, declarations and `@at-root`.

use super::*;

impl<'a> Visitor<'a> {
    pub(super) fn trim_included(&self, nodes: &[CssTreeIdx]) -> CssTreeIdx {
        if nodes.is_empty() {
            return CssTree::ROOT;
        }

        let mut parent = self.parent;

        let mut innermost_contiguous: Option<usize> = None;

        for i in 0..nodes.len() {
            while parent != nodes.get(i).copied() {
                innermost_contiguous = None;

                let grandparent = self.css_tree.child_to_parent.get(&parent.unwrap()).copied();
                if grandparent.is_none() {
                    unreachable!(
                        "Expected {:?} to be an ancestor of {:?}.",
                        nodes[i], grandparent
                    )
                }
                parent = grandparent;
            }
            innermost_contiguous = innermost_contiguous.or(Some(i));

            let grandparent = self.css_tree.child_to_parent.get(&parent.unwrap()).copied();
            if grandparent.is_none() {
                unreachable!(
                    "Expected {:?} to be an ancestor of {:?}.",
                    nodes[i], grandparent
                )
            }
            parent = grandparent;
        }

        if parent != Some(CssTree::ROOT) {
            return CssTree::ROOT;
        }

        nodes[innermost_contiguous.unwrap()]
    }

    pub(super) fn visit_at_root_rule(
        &mut self,
        mut at_root_rule: AstAtRootRule,
    ) -> SassResult<Option<Value>> {
        let query = match at_root_rule.query.clone() {
            Some(query) => {
                let resolved = self.perform_interpolation(query.node, true)?;

                let span = query.span;

                let query_toks = Lexer::new_from_string(&resolved, span);

                AtRootQueryParser::new(query_toks).parse()?
            }
            None => AtRootQuery::default(),
        };

        let mut current_parent_idx = self.parent;

        let mut included = Vec::new();

        while let Some(parent_idx) = current_parent_idx {
            let parent = self.css_tree.get(parent_idx);
            let grandparent_idx = match &*parent {
                Some(parent) => {
                    if !query.excludes(parent) {
                        included.push(parent_idx);
                    }
                    self.css_tree.child_to_parent.get(&parent_idx).copied()
                }
                None => break,
            };

            current_parent_idx = grandparent_idx;
        }

        let root = self.trim_included(&included);

        // If we didn't exclude any rules, we don't need to use the copies we might
        // have created.
        if Some(root) == self.parent {
            self.with_scope::<SassResult<()>, _>(false, true, |visitor| {
                for stmt in at_root_rule.body {
                    let result = visitor.visit_stmt(stmt)?;
                    debug_assert!(result.is_none());
                }

                Ok(())
            })?;
            return Ok(None);
        }

        let inner_copy = if !included.is_empty() {
            let inner_copy = self
                .css_tree
                .get(*included.first().unwrap())
                .as_ref()
                .map(CssStmt::copy_without_children);
            let mut outer_copy = self.css_tree.add_stmt(inner_copy.unwrap(), None);

            for node in &included[1..] {
                let copy = self
                    .css_tree
                    .get(*node)
                    .as_ref()
                    .map(CssStmt::copy_without_children)
                    .unwrap();

                let copy_idx = self.css_tree.add_stmt(copy, None);
                self.css_tree.link_child_to_parent(outer_copy, copy_idx);

                outer_copy = copy_idx;
            }

            Some(outer_copy)
        } else {
            let inner_copy = self
                .css_tree
                .get(root)
                .as_ref()
                .map(CssStmt::copy_without_children);
            inner_copy.map(|p| self.css_tree.add_stmt(p, None))
        };

        let body = mem::take(&mut at_root_rule.body);

        self.with_scope_for_at_root::<SassResult<()>, _>(inner_copy, &query, |visitor| {
            for stmt in body {
                let result = visitor.visit_stmt(stmt)?;
                debug_assert!(result.is_none());
            }

            Ok(())
        })?;

        Ok(None)
    }

    pub(super) fn with_scope_for_at_root<T, F: FnOnce(&mut Self) -> T>(
        &mut self,
        new_parent_idx: Option<CssTreeIdx>,
        query: &AtRootQuery,
        callback: F,
    ) -> T {
        let old_parent = self.parent;
        self.parent = new_parent_idx;

        let old_at_root_excluding_style_rule = self.flags.at_root_excluding_style_rule();

        if query.excludes_style_rules() {
            self.flags
                .set(ContextFlags::AT_ROOT_EXCLUDING_STYLE_RULE, true);
        }

        let old_media_query_info = if self.media_queries.is_some() && query.excludes_name("media") {
            Some((self.media_queries.take(), self.media_query_sources.take()))
        } else {
            None
        };

        let was_in_keyframes = if self.flags.in_keyframes() && query.excludes_name("keyframes") {
            let was = self.flags.in_keyframes();
            self.flags.set(ContextFlags::IN_KEYFRAMES, false);
            was
        } else {
            self.flags.in_keyframes()
        };

        // todo:
        // if self.flags.in_unknown_at_rule() && !included.iter().any(|parent| parent is CssAtRule)

        let res = self.with_scope(false, true, callback);

        self.parent = old_parent;

        self.flags.set(
            ContextFlags::AT_ROOT_EXCLUDING_STYLE_RULE,
            old_at_root_excluding_style_rule,
        );

        if let Some((old_media_queries, old_media_query_sources)) = old_media_query_info {
            self.media_queries = old_media_queries;
            self.media_query_sources = old_media_query_sources;
        }

        self.flags.set(ContextFlags::IN_KEYFRAMES, was_in_keyframes);

        res
    }

    pub(crate) fn visit_ruleset(&mut self, ruleset: AstRuleSet) -> SassResult<Option<Value>> {
        if self.declaration_name.is_some() {
            return Err((
                "Style rules may not be used within nested declarations.",
                ruleset.span,
            )
                .into());
        }

        let AstRuleSet {
            selector: ruleset_selector,
            body: ruleset_body,
            ..
        } = ruleset;

        let selector_text = self.interpolation_to_value(ruleset_selector, true, true)?;

        if self.flags.in_keyframes() {
            let span = ruleset.selector_span;
            let sel_toks = Lexer::new_from_string(&selector_text, span);
            let parsed_selector =
                KeyframesSelectorParser::new(sel_toks).parse_keyframes_selector()?;

            let keyframes_ruleset = CssStmt::KeyframesRuleSet(KeyframesRuleSet {
                selector: parsed_selector,
                body: Vec::new(),
            });

            self.with_parent(
                keyframes_ruleset,
                true,
                |visitor| {
                    for stmt in ruleset_body {
                        let result = visitor.visit_stmt(stmt)?;
                        debug_assert!(result.is_none());
                    }

                    Ok(())
                },
                CssStmt::is_style_rule,
            )?;

            return Ok(None);
        }

        let mut parsed_selector = if self.is_plain_css {
            let sel_toks = Lexer::new_from_string(&selector_text, ruleset.selector_span);
            SelectorParser::new(sel_toks, true, false, ruleset.selector_span)
                .plain_css()
                .parse()?
        } else {
            self.parse_selector_from_string(&selector_text, true, true, ruleset.selector_span)?
        };

        // As dart-sass: nest the selector within its parent's and write the rule after the parent,
        // unless the rule is written nested, as CSS nesting: a rule nested in plain CSS, or a plain
        // CSS rule with `&` in a Sass rule.
        let merge = if !self.style_rule_exists() {
            true
        } else if self.style_rule_from_plain_css {
            false
        } else {
            !(self.is_plain_css && parsed_selector.contains_parent_selector())
        };
        if merge {
            if self.is_plain_css
                && parsed_selector.components.iter().any(|complex| {
                    matches!(
                        complex.components.first(),
                        Some(ComplexSelectorComponent::Combinator(..))
                    )
                })
            {
                return Err((
                    "Top-level leading combinators aren't allowed in plain CSS.",
                    ruleset.selector_span,
                )
                    .into());
            }

            parsed_selector = parsed_selector.resolve_parent_selectors(
                self.style_rule_ignoring_at_root
                    .as_ref()
                    // todo: this clone should be superfluous(?)
                    .map(|x| x.as_selector_list().clone()),
                !self.flags.at_root_excluding_style_rule(),
                self.is_plain_css,
            )?;
        }

        // todo: _mediaQueries
        let selector = self
            .extender
            .add_selector(parsed_selector, &self.media_queries);

        let rule = CssStmt::RuleSet {
            selector: selector.clone(),
            body: Vec::new(),
            is_group_end: false,
        };

        let old_at_root_excluding_style_rule = self.flags.at_root_excluding_style_rule();

        self.flags
            .set(ContextFlags::AT_ROOT_EXCLUDING_STYLE_RULE, false);

        let old_style_rule_ignoring_at_root = self.style_rule_ignoring_at_root.take();
        self.style_rule_ignoring_at_root = Some(selector);
        let old_style_rule_from_plain_css =
            mem::replace(&mut self.style_rule_from_plain_css, self.is_plain_css);
        let old_style_rule_idx = self.style_rule_idx;

        // A merged rule goes after its parent rules; a nested one into its parent.
        let through: fn(&CssStmt) -> bool = if merge {
            CssStmt::is_style_rule
        } else {
            |_| false
        };
        self.with_parent(
            rule,
            true,
            |visitor| {
                visitor.style_rule_idx = visitor.parent;
                for stmt in ruleset_body {
                    let result = visitor.visit_stmt(stmt)?;
                    debug_assert!(result.is_none());
                }

                Ok(())
            },
            through,
        )?;

        self.style_rule_ignoring_at_root = old_style_rule_ignoring_at_root;
        self.style_rule_from_plain_css = old_style_rule_from_plain_css;
        self.style_rule_idx = old_style_rule_idx;
        self.flags.set(
            ContextFlags::AT_ROOT_EXCLUDING_STYLE_RULE,
            old_at_root_excluding_style_rule,
        );

        self.set_group_end();

        Ok(None)
    }

    pub(crate) fn visit_style(&mut self, style: AstStyle) -> SassResult<Option<Value>> {
        if !self.style_rule_exists()
            && !self.flags.in_unknown_at_rule()
            && !self.flags.in_keyframes()
        {
            return Err((
                "Declarations may only be used within style rules.",
                style.span,
            )
                .into());
        }

        let is_custom_property = style.is_custom_property();

        let mut name = self.interpolation_to_value(style.name, false, true)?;

        if let Some(declaration_name) = &self.declaration_name {
            name = format!("{}-{}", declaration_name, name);
        }

        if let Some(value) = style
            .value
            .map(|s| {
                SassResult::Ok(Spanned {
                    node: self.visit_expr(s.node)?,
                    span: s.span,
                })
            })
            .transpose()?
        {
            // If the value is an empty list, preserve it, because converting it to CSS
            // will throw an error that we want the user to see. Custom properties are allowed
            // to have empty values, per spec.
            if !value.is_blank() || value.is_empty_list() || name.starts_with("--") {
                self.copy_parent_after_sibling();
                // todo: superfluous clones?
                self.css_tree.add_stmt(
                    CssStmt::Style(Style {
                        property: InternedString::get_or_intern(&name),
                        value: Box::new(value),
                        declared_as_custom_property: is_custom_property,
                        name_span: style.span,
                    }),
                    self.parent,
                );
            }
        }

        let children = style.body;

        if !children.is_empty() {
            let old_declaration_name = self.declaration_name.take();
            self.declaration_name = Some(name);
            self.with_scope::<SassResult<()>, _>(false, true, |visitor| {
                for stmt in children {
                    let result = visitor.visit_stmt(stmt)?;
                    debug_assert!(result.is_none());
                }

                Ok(())
            })?;
            self.declaration_name = old_declaration_name;
        }

        Ok(None)
    }
}
