//! The auto-expand size gate.
//!
//! Auto-expand answers one question at the hook: is this entry small enough to
//! stand in place of its own marker? It runs the same cycle-safe walk as an
//! explicit retrieve and then refuses the result if it does not fit the limit. It
//! is not a second resolver - every resolution rule lives in `Recursive::Fn`, and
//! a divergence between the two would mean the same hash expands differently
//! depending on who asked.

use std::collections::HashMap;

use crate::State::Struct::AphroditeState;
use super::Recursive;

/// Resolve `HashValue` and every nested marker, but only when the result fits.
///
/// The contract for auto-expand: same `Visited` set, same `state.recursive_depth`
/// limit, and the resolved content returned ONLY when it fits within `Limit` bytes
/// (pre-Rust `_resolve.py` + `_core/config.py` AUTO_EXPAND_LIMIT). No marker is
/// re-emitted: when the content fits, the returned string IS the fully-resolved
/// content.
///
/// Over-limit results return `None` so the caller keeps the original marker - the
/// stored bytes stay retrievable for a later, explicit resolve. A `None` is also
/// returned when the top-level hash cannot be resolved at all, deliberately
/// indistinguishable from an over-limit result: the caller's marker fallback is
/// correct for both, and telling them apart would only invite a caller to guess.
pub fn Fn(
	State:&mut AphroditeState,
	HashValue:&str,
	Limit:usize,
	Resolved:&mut HashMap<String, String>,
	Visited:&mut Vec<String>,
) -> Option<String> {
	let Content = Recursive::Fn(State, HashValue, 0, Resolved, Visited)?;
	match Content.len() <= Limit {
		true => Some(Content),
		false => None,
	}
}
