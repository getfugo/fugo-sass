//! Interned strings: each distinct string is stored once, for the life of the process, and an
//! `InternedString` is its index.
//!
//! The strings are shared by every thread, so an `InternedString` means the same string on any
//! thread, and resolving one gives a `&'static str`.

use std::{
    collections::HashMap,
    fmt::{self, Display},
    sync::{LazyLock, PoisonError, RwLock},
};

/// The interned strings, and the index of each.
#[derive(Default)]
struct Interner {
    strings: Vec<&'static str>,
    indexes: HashMap<&'static str, u32>,
}

static INTERNER: LazyLock<RwLock<Interner>> = LazyLock::new(RwLock::default);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct InternedString(u32);

impl InternedString {
    pub fn get_or_intern<T: AsRef<str>>(s: T) -> Self {
        let s = s.as_ref();
        if let Some(&index) = INTERNER
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .indexes
            .get(s)
        {
            return Self(index);
        }

        let mut interner = INTERNER.write().unwrap_or_else(PoisonError::into_inner);
        // Another thread may have interned it since the read.
        if let Some(&index) = interner.indexes.get(s) {
            return Self(index);
        }
        // Interned strings live as long as the process.
        let string: &'static str = Box::leak(s.to_owned().into_boxed_str());
        let index =
            u32::try_from(interner.strings.len()).expect("fewer than 2^32 interned strings");
        interner.strings.push(string);
        interner.indexes.insert(string, index);
        Self(index)
    }

    #[allow(dead_code)]
    pub fn resolve(self) -> String {
        self.resolve_ref().to_owned()
    }

    #[allow(dead_code)]
    pub fn is_empty(self) -> bool {
        self.resolve_ref().is_empty()
    }

    pub fn resolve_ref(self) -> &'static str {
        INTERNER
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .strings[self.0 as usize]
    }
}

impl Display for InternedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.resolve_ref())
    }
}
