//! `@media`, `@supports` and unknown at-rules.

use super::*;

impl<'a> Visitor<'a> {
    pub(super) fn parenthesize_supports_condition(
        &mut self,
        condition: AstSupportsCondition,
        operator: Option<&str>,
    ) -> SassResult<String> {
        match &condition {
            AstSupportsCondition::Negation(..) => {
                Ok(format!("({})", self.visit_supports_condition(condition)?))
            }
            AstSupportsCondition::Operation {
                operator: operator2,
                ..
            } if operator2.is_none() || operator2.as_deref() != operator => {
                Ok(format!("({})", self.visit_supports_condition(condition)?))
            }
            _ => self.visit_supports_condition(condition),
        }
    }

    pub(super) fn visit_supports_condition(
        &mut self,
        condition: AstSupportsCondition,
    ) -> SassResult<String> {
        match condition {
            AstSupportsCondition::Operation {
                left,
                operator,
                right,
            } => Ok(format!(
                "{} {} {}",
                self.parenthesize_supports_condition(*left, operator.as_deref())?,
                operator.as_ref().unwrap(),
                self.parenthesize_supports_condition(*right, operator.as_deref())?
            )),
            AstSupportsCondition::Negation(condition) => Ok(format!(
                "not {}",
                self.parenthesize_supports_condition(*condition, None)?
            )),
            AstSupportsCondition::Interpolation(expr) => {
                self.evaluate_to_css(expr, QuoteKind::None, self.empty_span)
            }
            AstSupportsCondition::Declaration { name, value } => {
                let old_in_supports_decl = self.flags.in_supports_declaration();
                self.flags.set(ContextFlags::IN_SUPPORTS_DECLARATION, true);

                let is_custom_property = match &name {
                    AstExpr::String(StringExpr(text, QuoteKind::None), ..) => {
                        text.initial_plain().starts_with("--")
                    }
                    _ => false,
                };

                let result = format!(
                    "({}:{}{})",
                    self.evaluate_to_css(name, QuoteKind::Quoted, self.empty_span)?,
                    if is_custom_property { "" } else { " " },
                    self.evaluate_to_css(value, QuoteKind::Quoted, self.empty_span)?,
                );

                self.flags
                    .set(ContextFlags::IN_SUPPORTS_DECLARATION, old_in_supports_decl);

                Ok(result)
            }
            AstSupportsCondition::Function { name, args } => Ok(format!(
                "{}({})",
                self.perform_interpolation(name, false)?,
                self.perform_interpolation(args, false)?
            )),
            AstSupportsCondition::Anything { contents } => Ok(format!(
                "({})",
                self.perform_interpolation(contents, false)?,
            )),
        }
    }

    pub(super) fn visit_supports_rule(&mut self, supports_rule: AstSupportsRule) -> SassResult<()> {
        if self.declaration_name.is_some() {
            return Err((
                "Supports rules may not be used within nested declarations.",
                supports_rule.span,
            )
                .into());
        }

        let condition = self.visit_supports_condition(supports_rule.condition)?;

        let css_supports_rule = CssStmt::Supports(
            SupportsRule {
                params: condition,
                body: Vec::new(),
            },
            false,
        );

        let children = supports_rule.body;

        // In plain CSS nesting, the rule is written where it is (see `visit_media_rule`).
        if self.has_css_nesting() {
            return self.with_parent(
                css_supports_rule,
                true,
                |visitor| {
                    for stmt in children {
                        visitor.visit_stmt(stmt)?;
                    }
                    Ok(())
                },
                |_| false,
            );
        }

        self.with_parent(
            css_supports_rule,
            true,
            |visitor| {
                if !visitor.style_rule_exists() {
                    for stmt in children {
                        let result = visitor.visit_stmt(stmt)?;
                        debug_assert!(result.is_none());
                    }
                } else {
                    // If we're in a style rule, copy it into the supports rule so that
                    // declarations immediately inside @supports have somewhere to go.
                    //
                    // For example, "a {@supports (a: b) {b: c}}" should produce "@supports
                    // (a: b) {a {b: c}}".
                    let selector = visitor.style_rule_ignoring_at_root.clone().unwrap();
                    let ruleset = CssStmt::RuleSet {
                        selector,
                        body: Vec::new(),
                        is_group_end: false,
                    };

                    visitor.with_parent(
                        ruleset,
                        false,
                        |visitor| {
                            for stmt in children {
                                let result = visitor.visit_stmt(stmt)?;
                                debug_assert!(result.is_none());
                            }

                            Ok(())
                        },
                        |_| false,
                    )?;
                }

                Ok(())
            },
            CssStmt::is_style_rule,
        )?;

        Ok(())
    }

    pub(super) fn merge_media_queries(
        queries1: &[MediaQuery],
        queries2: &[MediaQuery],
    ) -> Option<Vec<MediaQuery>> {
        let mut queries = Vec::new();

        for query1 in queries1 {
            for query2 in queries2 {
                match query1.merge(query2) {
                    MediaQueryMergeResult::Empty => continue,
                    MediaQueryMergeResult::Unrepresentable => return None,
                    MediaQueryMergeResult::Success(result) => queries.push(result),
                }
            }
        }

        Some(queries)
    }

    pub(super) fn visit_media_queries(
        &mut self,
        queries: Interpolation,
        span: Span,
    ) -> SassResult<Vec<CssMediaQuery>> {
        let resolved = self.perform_interpolation(queries, true)?;

        CssMediaQuery::parse_list(&resolved, span)
    }

    pub(super) fn visit_media_rule(&mut self, media_rule: AstMedia) -> SassResult<Option<Value>> {
        if self.declaration_name.is_some() {
            return Err((
                "Media rules may not be used within nested declarations.",
                media_rule.span,
            )
                .into());
        }

        let queries1 = self.visit_media_queries(media_rule.query, media_rule.query_span)?;

        // In plain CSS nesting, the rule is written where it is, without merging or bubbling:
        // only browsers that support nesting can use it anyway.
        if self.has_css_nesting() {
            let rule = CssStmt::Media(
                MediaRule {
                    query: queries1,
                    body: Vec::new(),
                },
                false,
            );
            self.with_parent(
                rule,
                false,
                |visitor| {
                    for stmt in media_rule.body {
                        visitor.visit_stmt(stmt)?;
                    }
                    Ok(())
                },
                |_| false,
            )?;
            return Ok(None);
        }
        // todo: superfluous clone?
        let queries2 = self.media_queries.clone();
        let merged_queries = queries2
            .as_ref()
            .and_then(|queries2| Self::merge_media_queries(queries2, &queries1));

        let merged_sources = match &merged_queries {
            Some(merged_queries) if merged_queries.is_empty() => return Ok(None),
            Some(..) => {
                let mut set = IndexSet::new();
                set.extend(self.media_query_sources.clone().unwrap());
                set.extend(self.media_queries.clone().unwrap());
                set.extend(queries1.clone());
                set
            }
            None => IndexSet::new(),
        };

        let children = media_rule.body;

        let query = merged_queries.clone().unwrap_or_else(|| queries1.clone());

        let media_rule = CssStmt::Media(
            MediaRule {
                query,
                body: Vec::new(),
            },
            false,
        );

        self.with_parent(
            media_rule,
            true,
            |visitor| {
                visitor.with_media_queries(
                    Some(merged_queries.unwrap_or(queries1)),
                    Some(merged_sources.clone()),
                    |visitor| {
                        if !visitor.style_rule_exists() {
                            for stmt in children {
                                let result = visitor.visit_stmt(stmt)?;
                                debug_assert!(result.is_none());
                            }
                        } else {
                            // If we're in a style rule, copy it into the media query so that
                            // declarations immediately inside @media have somewhere to go.
                            //
                            // For example, "a {@media screen {b: c}}" should produce
                            // "@media screen {a {b: c}}".
                            let selector = visitor.style_rule_ignoring_at_root.clone().unwrap();
                            let ruleset = CssStmt::RuleSet {
                                selector,
                                body: Vec::new(),
                                is_group_end: false,
                            };

                            visitor.with_parent(
                                ruleset,
                                false,
                                |visitor| {
                                    for stmt in children {
                                        let result = visitor.visit_stmt(stmt)?;
                                        debug_assert!(result.is_none());
                                    }

                                    Ok(())
                                },
                                |_| false,
                            )?;
                        }

                        Ok(())
                    },
                )
            },
            |stmt| match stmt {
                CssStmt::RuleSet { .. } => true,
                // todo: node.queries.every(mergedSources.contains))
                CssStmt::Media(media_rule, ..) => {
                    !merged_sources.is_empty()
                        && media_rule
                            .query
                            .iter()
                            .all(|query| merged_sources.contains(query))
                }
                _ => false,
            },
        )?;

        Ok(None)
    }

    pub(super) fn visit_unknown_at_rule(
        &mut self,
        unknown_at_rule: AstUnknownAtRule,
    ) -> SassResult<Option<Value>> {
        if self.declaration_name.is_some() {
            return Err((
                "At-rules may not be used within nested declarations.",
                unknown_at_rule.span,
            )
                .into());
        }

        let name = self.interpolation_to_value(unknown_at_rule.name, false, false)?;

        let value = unknown_at_rule
            .value
            .map(|v| self.interpolation_to_value(v, true, true))
            .transpose()?;

        if unknown_at_rule.body.is_none() {
            let stmt = CssStmt::UnknownAtRule(
                UnknownAtRule {
                    name,
                    params: value.unwrap_or_default(),
                    body: Vec::new(),
                    has_body: false,
                },
                false,
            );

            self.copy_parent_after_sibling();
            self.css_tree.add_stmt(stmt, self.parent);

            return Ok(None);
        }

        let was_in_keyframes = self.flags.in_keyframes();
        let was_in_unknown_at_rule = self.flags.in_unknown_at_rule();

        if unvendor(&name) == "keyframes" {
            self.flags.set(ContextFlags::IN_KEYFRAMES, true);
        } else {
            self.flags.set(ContextFlags::IN_UNKNOWN_AT_RULE, true);
        }

        let children = unknown_at_rule.body.unwrap();

        // `@font-face` in a style rule is pulled out to the root (dart-sass 1.58.4).
        let is_font_face = name == "font-face";

        let stmt = CssStmt::UnknownAtRule(
            UnknownAtRule {
                name,
                params: value.unwrap_or_default(),
                body: Vec::new(),
                has_body: true,
            },
            false,
        );

        // In plain CSS nesting, the rule is written where it is (see `visit_media_rule`).
        if self.has_css_nesting() {
            self.with_parent(
                stmt,
                true,
                |visitor| {
                    for stmt in children {
                        visitor.visit_stmt(stmt)?;
                    }
                    Ok(())
                },
                |_| false,
            )?;
            self.flags.set(ContextFlags::IN_KEYFRAMES, was_in_keyframes);
            self.flags
                .set(ContextFlags::IN_UNKNOWN_AT_RULE, was_in_unknown_at_rule);
            return Ok(None);
        }

        self.with_parent(
            stmt,
            true,
            |visitor| {
                if !visitor.style_rule_exists() || visitor.flags.in_keyframes() || is_font_face {
                    for stmt in children {
                        let result = visitor.visit_stmt(stmt)?;
                        debug_assert!(result.is_none());
                    }
                } else {
                    // If we're in a style rule, copy it into the at-rule so that
                    // declarations immediately inside it have somewhere to go.
                    //
                    // For example, "a {@foo {b: c}}" should produce "@foo {a {b: c}}".
                    let selector = visitor.style_rule_ignoring_at_root.clone().unwrap();

                    let style_rule = CssStmt::RuleSet {
                        selector,
                        body: Vec::new(),
                        is_group_end: false,
                    };

                    visitor.with_parent(
                        style_rule,
                        false,
                        |visitor| {
                            for stmt in children {
                                let result = visitor.visit_stmt(stmt)?;
                                debug_assert!(result.is_none());
                            }

                            Ok(())
                        },
                        |_| false,
                    )?;
                }

                Ok(())
            },
            CssStmt::is_style_rule,
        )?;

        self.flags.set(ContextFlags::IN_KEYFRAMES, was_in_keyframes);
        self.flags
            .set(ContextFlags::IN_UNKNOWN_AT_RULE, was_in_unknown_at_rule);

        Ok(None)
    }
}
