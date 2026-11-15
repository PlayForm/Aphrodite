//! The per-session-tree flag path.
//!
//! One file per ROOT session id. Subagents never get their own file: they
//! resolve to the root, so a single toggle covers the tree and a sibling tree
//! stays quiet.

use std::path::PathBuf;

use crate::Debug::{Flag, Runtime};

/// Path of the scoped debug flag for `session`.
///
/// `session` is expected to be a ROOT id by the time it gets here - the
/// resolution walks the chain first. It does not check existence and does not
/// validate the id, so an empty id yields `debug.` (a harmless file nobody
/// reads).
pub(crate) fn Fn(session:&str) -> PathBuf {
	Runtime::Fn::Fn().join(format!("{}.{}", Flag::Const::FLAG_PREFIX, session))
}
