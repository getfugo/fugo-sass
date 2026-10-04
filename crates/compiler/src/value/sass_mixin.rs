use std::fmt;

use crate::ast::Mixin;

/// A mixin reference, which `meta.get-mixin()` and `meta.module-mixins()` return, and
/// `meta.apply()` runs.
///
/// Two references are equal when they are to the same mixin.
#[derive(Clone, PartialEq, Eq)]
pub struct SassMixin(pub(crate) Mixin);

impl SassMixin {
    /// The name of the mixin.
    pub fn name(&self) -> String {
        self.0.name()
    }
}

impl fmt::Debug for SassMixin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("SassMixin").field(&self.name()).finish()
    }
}
