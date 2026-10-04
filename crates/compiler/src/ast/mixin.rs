use std::{fmt, sync::Arc};

use crate::{
    ast::ArgumentResult,
    error::SassResult,
    evaluate::{Environment, Visitor},
};

pub(crate) type BuiltinMixin = fn(ArgumentResult, &mut Visitor) -> SassResult<()>;

pub(crate) use crate::ast::AstMixin as UserDefinedMixin;

/// A mixin, which `@include` and `meta.apply()` run, and `meta.get-mixin()` returns.
#[derive(Clone)]
pub(crate) enum Mixin {
    /// A `@mixin` rule. Each evaluation of the rule is a different mixin.
    UserDefined(Arc<UserDefinedMixin>, Environment),
    Builtin {
        name: &'static str,
        mixin: BuiltinMixin,
        /// Whether `@include` may pass the mixin a content block.
        accepts_content: bool,
    },
}

impl Mixin {
    pub fn name(&self) -> String {
        match self {
            Self::UserDefined(mixin, ..) => mixin.name.to_string(),
            Self::Builtin { name, .. } => (*name).to_owned(),
        }
    }

    /// Whether the mixin takes a content block (it has `@content`, for a user-defined one).
    pub fn accepts_content(&self) -> bool {
        match self {
            Self::UserDefined(mixin, ..) => mixin.has_content,
            Self::Builtin {
                accepts_content, ..
            } => *accepts_content,
        }
    }
}

/// Mixins are equal when they are the same mixin, as `dart-sass` compares them.
impl PartialEq for Mixin {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::UserDefined(a, ..), Self::UserDefined(b, ..)) => Arc::ptr_eq(a, b),
            (Self::Builtin { name: a, .. }, Self::Builtin { name: b, .. }) => a == b,
            _ => false,
        }
    }
}

impl Eq for Mixin {}

impl fmt::Debug for Mixin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserDefined(u, ..) => f
                .debug_struct("AstMixin")
                .field("name", &u.name)
                .field("args", &u.args)
                .field("body", &u.body)
                .field("has_content", &u.has_content)
                .finish(),
            Self::Builtin { name, .. } => {
                f.debug_struct("BuiltinMixin").field("name", name).finish()
            }
        }
    }
}
