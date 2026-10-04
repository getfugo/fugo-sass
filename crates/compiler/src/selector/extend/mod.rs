use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
};

use codemap::Span;

use indexmap::IndexMap;

use crate::{ast::CssMediaQuery, error::SassResult};

use super::{
    ComplexSelector, ComplexSelectorComponent, ComplexSelectorHashSet, CompoundSelector, Pseudo,
    SelectorList, SimpleSelector,
};

pub(crate) use extended_selector::ExtendedSelector;
use extended_selector::SelectorHashSet;
pub(crate) use extension::Extension;
pub(crate) use functions::unify_complex;
use functions::{paths, weave};
use merged::MergedExtension;
pub(crate) use rule::ExtendRule;

mod extended_selector;
mod extending;
mod extension;
mod functions;
mod merged;
mod pseudo;
mod rule;
mod store;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
/// Different modes in which extension can run.
#[derive(Default)]
enum ExtendMode {
    /// Normal mode, used with the `@extend` rule.
    ///
    /// This preserves existing selectors and extends each target individually.
    #[default]
    Normal,

    /// Replace mode, used by the `selector-replace()` function.
    ///
    /// This replaces existing selectors and requires every target to match to
    /// extend a given compound selector.
    Replace,

    /// All-targets mode, used by the `selector-extend()` function.
    ///
    /// This preserves existing selectors but requires every target to match to
    /// extend a given compound selector.
    AllTargets,
}

#[derive(Clone, Debug)]
pub(crate) struct ExtensionStore {
    /// A map from all simple selectors in the stylesheet to the selector lists
    /// that contain them.
    ///
    /// This is used to find which selectors an `@extend` applies to and adjust
    /// them.
    selectors: HashMap<SimpleSelector, SelectorHashSet>,

    /// A map from all extended simple selectors to the sources of those
    /// extensions.
    extensions: HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>>,

    /// A map from all simple selectors in extenders to the extensions that those
    /// extenders define.
    extensions_by_extender: HashMap<SimpleSelector, Vec<Extension>>,

    /// A map from the style rules' selectors (by identity, as dart-sass keys its boxes) to the
    /// media query contexts they're defined in.
    ///
    /// This tracks the contexts in which each selector's style rule is defined.
    /// If a rule is defined at the top level, it doesn't have an entry.
    media_contexts: HashMap<ExtendedSelector, Vec<CssMediaQuery>>,

    /// A map from `SimpleSelector`s to the specificity of their source
    /// selectors.
    ///
    /// This tracks the maximum specificity of the `ComplexSelector` that
    /// originally contained each `SimpleSelector`. This allows us to ensure that
    /// we don't trim any selectors that need to exist to satisfy the [second law
    /// of extend][].
    ///
    /// [second law of extend]: https://github.com/sass/sass/issues/324#issuecomment-4607184
    source_specificity: HashMap<SimpleSelector, i32>,

    /// A set of `ComplexSelector`s that were originally part of
    /// their component `SelectorList`s, as opposed to being added by `@extend`.
    ///
    /// This allows us to ensure that we don't trim any selectors that need to
    /// exist to satisfy the [first law of extend][].
    ///
    /// [first law of extend]: https://github.com/sass/sass/issues/324#issuecomment-4607184
    originals: ComplexSelectorHashSet,

    /// The mode that controls this extender's behavior.
    mode: ExtendMode,

    span: Span,
}

impl ExtensionStore {
    /// An `Extender` that contains no extensions and can have no extensions added.
    // TODO: empty extender
    #[allow(dead_code)]
    const EMPTY: () = ();

    pub fn extend(
        selector: SelectorList,
        source: SelectorList,
        targets: SelectorList,
        span: Span,
    ) -> SassResult<SelectorList> {
        Self::extend_or_replace(selector, source, targets, ExtendMode::AllTargets, span)
    }

    pub fn new(span: Span) -> Self {
        Self {
            selectors: HashMap::new(),
            extensions: HashMap::new(),
            extensions_by_extender: HashMap::new(),
            media_contexts: HashMap::new(),
            source_specificity: HashMap::new(),
            originals: ComplexSelectorHashSet::new(),
            mode: ExtendMode::Normal,
            span,
        }
    }

    pub fn replace(
        selector: SelectorList,
        source: SelectorList,
        targets: SelectorList,
        span: Span,
    ) -> SassResult<SelectorList> {
        Self::extend_or_replace(selector, source, targets, ExtendMode::Replace, span)
    }

    fn extend_or_replace(
        selector: SelectorList,
        source: SelectorList,
        targets: SelectorList,
        mode: ExtendMode,
        span: Span,
    ) -> SassResult<SelectorList> {
        let extenders: IndexMap<ComplexSelector, Extension> = source
            .components
            .into_iter()
            .map(|complex| {
                (
                    complex.clone(),
                    Extension::one_off(complex, None, false, span),
                )
            })
            .collect();

        let compound_targets = targets
            .components
            .into_iter()
            .map(|complex| {
                if complex.components.len() == 1 {
                    Ok(complex.components.first().unwrap().as_compound().clone())
                } else {
                    Err((format!("Can't extend complex selector {}.", complex), span).into())
                }
            })
            .collect::<SassResult<Vec<CompoundSelector>>>()?;

        let extensions: HashMap<SimpleSelector, IndexMap<ComplexSelector, Extension>> =
            compound_targets
                .into_iter()
                .flat_map(|compound| {
                    compound
                        .components
                        .into_iter()
                        .map(|simple| (simple, extenders.clone()))
                })
                .collect();

        let mut extender = ExtensionStore::with_mode(mode, span);

        if !selector.is_invisible() {
            extender.originals.extend(selector.components.iter());
        }

        Ok(extender.extend_list(selector, Some(&extensions), &None))
    }

    fn with_mode(mode: ExtendMode, span: Span) -> Self {
        Self {
            mode,
            ..ExtensionStore::new(span)
        }
    }

    /// Adds an extension to this extender.
    ///
    /// The `extender` is the selector for the style rule in which the extension
    /// is defined, and `target` is the selector passed to `@extend`. The `extend`
    /// provides the extend span and indicates whether the extension is optional.
    ///
    /// The `media_context` defines the media query context in which the extension
    /// is defined. It can only extend selectors within the same context. A `None`
    /// context indicates no media queries.
    /// Whether no `@extend` was added to this store.
    pub fn is_empty(&self) -> bool {
        self.extensions.is_empty()
    }

    /// The simple selectors in the style rules this store extends.
    pub fn simple_selectors(&self) -> HashSet<SimpleSelector> {
        self.selectors.keys().cloned().collect()
    }
}

/// Rotates the element in list from `start` (inclusive) to `end` (exclusive)
/// one index higher, looping the final element back to `start`.
fn rotate_slice<T: Clone>(list: &mut VecDeque<T>, start: usize, end: usize) {
    let mut element = list.get(end - 1).unwrap().clone();
    for i in start..end {
        let next = list.get(i).unwrap().clone();
        list[i] = element;
        element = next;
    }
}

/// Like `HashMap::extend`, but for two-layer maps.
///
/// This avoids copying inner maps from `source` if possible.
fn map_add_all_2<K1: Hash + Eq, K2: Hash + Eq, V>(
    destination: &mut HashMap<K1, IndexMap<K2, V>>,
    source: HashMap<K1, IndexMap<K2, V>>,
) {
    for (key, inner) in source {
        match destination.get_mut(&key) {
            Some(existing) => existing.extend(inner),
            None => {
                destination.insert(key, inner);
            }
        }
    }
}

/// The simple selectors in `complex`, those in pseudo-classes' selectors included.
fn simple_selectors(complex: &ComplexSelector) -> Vec<SimpleSelector> {
    let mut result = Vec::new();
    for component in &complex.components {
        if let ComplexSelectorComponent::Compound(compound) = component {
            for simple in &compound.components {
                result.push(simple.clone());
                if let SimpleSelector::Pseudo(Pseudo {
                    selector: Some(selector),
                    ..
                }) = simple
                {
                    for complex in &selector.components {
                        result.extend(simple_selectors(complex));
                    }
                }
            }
        }
    }
    result
}
