//! The registry accessor.

use std::sync::RwLock;

use crate::Template::State::{Const::Templates, Struct::Struct};

/// The registry, initialised empty (no templates, no family) on first use.
///
/// An empty registry is the documented default: no `[templates]` section means
/// the hardcoded per-arm builders stay in charge. That is why the cell starts
/// with `None`/`None` rather than a default family - a default family with no
/// templates would still answer `None`, but it would make the "is anything
/// configured" question harder to read in a debug line.
pub(crate) fn Fn() -> &'static RwLock<Struct> {
	Templates.get_or_init(|| RwLock::new(Struct { Templates:None, Family:None }))
}
