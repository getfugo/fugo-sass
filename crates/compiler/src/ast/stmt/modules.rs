//! `@use`, `@forward` and `@import`, and module configurations.

use super::*;

#[derive(Debug, Clone)]
pub struct AstPlainCssImport {
    pub url: Interpolation,
    pub modifiers: Option<Interpolation>,
    #[allow(unused)]
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstSassImport {
    pub url: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct AstImportRule {
    pub imports: Vec<AstImport>,
}

#[derive(Debug, Clone)]
pub enum AstImport {
    Plain(AstPlainCssImport),
    Sass(AstSassImport),
}

impl AstImport {
    pub fn is_dynamic(&self) -> bool {
        matches!(self, AstImport::Sass(..))
    }
}

#[derive(Debug, Clone)]
pub struct AstUseRule {
    pub url: PathBuf,
    pub namespace: Option<String>,
    pub configuration: Vec<ConfiguredVariable>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct ConfiguredVariable {
    pub name: Spanned<Identifier>,
    pub expr: Spanned<AstExpr>,
    pub is_guarded: bool,
}

#[derive(Debug, Clone)]
pub struct Configuration {
    pub(crate) values: Arc<dyn MapView<Value = ConfiguredValue>>,
    #[allow(unused)]
    pub(crate) original_config: Option<Rc<RefCell<Self>>>,
    pub(crate) span: Option<Span>,
}

impl Configuration {
    pub fn through_forward(
        config: Rc<RefCell<Self>>,
        forward: &AstForwardRule,
    ) -> Rc<RefCell<Self>> {
        if (*config).borrow().is_empty() {
            return Rc::new(RefCell::new(Configuration::empty()));
        }

        let mut new_values = Arc::clone(&(*config).borrow().values);

        // Only allow variables that are visible through the `@forward` to be
        // configured. These views support [Map.remove] so we can mark when a
        // configuration variable is used by removing it even when the underlying
        // map is wrapped.
        if let Some(prefix) = &forward.prefix {
            new_values = Arc::new(UnprefixedMapView(new_values, prefix.clone()));
        }

        if let Some(shown_variables) = &forward.shown_variables {
            new_values = Arc::new(LimitedMapView::safelist(new_values, shown_variables));
        } else if let Some(hidden_variables) = &forward.hidden_variables {
            new_values = Arc::new(LimitedMapView::blocklist(new_values, hidden_variables));
        }

        Rc::new(RefCell::new(Self::with_values(
            config,
            Arc::clone(&new_values),
        )))
    }

    fn with_values(
        config: Rc<RefCell<Self>>,
        values: Arc<dyn MapView<Value = ConfiguredValue>>,
    ) -> Self {
        Self {
            values,
            original_config: Some(config),
            span: None,
        }
    }

    pub fn first(&self) -> Option<Spanned<Identifier>> {
        let name = *self.values.keys().first()?;
        let value = self.values.get(name)?;

        Some(Spanned {
            node: name,
            span: value.configuration_span?,
        })
    }

    pub fn remove(&mut self, name: Identifier) -> Option<ConfiguredValue> {
        self.values.remove(name)
    }

    pub fn is_implicit(&self) -> bool {
        self.span.is_none()
    }

    pub fn implicit(values: IndexMap<Identifier, ConfiguredValue>) -> Self {
        Self {
            values: Arc::new(BaseMapView(Arc::new(RefCell::new(values)))),
            original_config: None,
            span: None,
        }
    }

    pub fn explicit(values: IndexMap<Identifier, ConfiguredValue>, span: Span) -> Self {
        Self {
            values: Arc::new(BaseMapView(Arc::new(RefCell::new(values)))),
            original_config: None,
            span: Some(span),
        }
    }

    pub fn empty() -> Self {
        Self {
            values: Arc::new(BaseMapView(Arc::new(RefCell::new(IndexMap::new())))),
            original_config: None,
            span: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    #[allow(unused)]
    pub fn original_config(config: Rc<RefCell<Configuration>>) -> Rc<RefCell<Configuration>> {
        match (*config).borrow().original_config.as_ref() {
            Some(v) => Rc::clone(v),
            None => Rc::clone(&config),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfiguredValue {
    pub value: Value,
    pub configuration_span: Option<Span>,
}

impl ConfiguredValue {
    pub fn explicit(value: Value, configuration_span: Span) -> Self {
        Self {
            value,
            configuration_span: Some(configuration_span),
        }
    }

    pub fn implicit(value: Value) -> Self {
        Self {
            value,
            configuration_span: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AstForwardRule {
    pub url: PathBuf,
    pub shown_mixins_and_functions: Option<IndexSet<Identifier>>,
    pub shown_variables: Option<IndexSet<Identifier>>,
    pub hidden_mixins_and_functions: Option<IndexSet<Identifier>>,
    pub hidden_variables: Option<IndexSet<Identifier>>,
    pub prefix: Option<String>,
    pub configuration: Vec<ConfiguredVariable>,
    pub span: Span,
}

impl AstForwardRule {
    pub fn new(
        url: PathBuf,
        prefix: Option<String>,
        configuration: Option<Vec<ConfiguredVariable>>,
        span: Span,
    ) -> Self {
        Self {
            url,
            shown_mixins_and_functions: None,
            shown_variables: None,
            hidden_mixins_and_functions: None,
            hidden_variables: None,
            prefix,
            configuration: configuration.unwrap_or_default(),
            span,
        }
    }

    pub fn show(
        url: PathBuf,
        shown_mixins_and_functions: IndexSet<Identifier>,
        shown_variables: IndexSet<Identifier>,
        prefix: Option<String>,
        configuration: Option<Vec<ConfiguredVariable>>,
        span: Span,
    ) -> Self {
        Self {
            url,
            shown_mixins_and_functions: Some(shown_mixins_and_functions),
            shown_variables: Some(shown_variables),
            hidden_mixins_and_functions: None,
            hidden_variables: None,
            prefix,
            configuration: configuration.unwrap_or_default(),
            span,
        }
    }

    pub fn hide(
        url: PathBuf,
        hidden_mixins_and_functions: IndexSet<Identifier>,
        hidden_variables: IndexSet<Identifier>,
        prefix: Option<String>,
        configuration: Option<Vec<ConfiguredVariable>>,
        span: Span,
    ) -> Self {
        Self {
            url,
            shown_mixins_and_functions: None,
            shown_variables: None,
            hidden_mixins_and_functions: Some(hidden_mixins_and_functions),
            hidden_variables: Some(hidden_variables),
            prefix,
            configuration: configuration.unwrap_or_default(),
            span,
        }
    }
}
