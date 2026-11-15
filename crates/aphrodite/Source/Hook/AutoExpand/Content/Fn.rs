//! Resolve a freshly stored hash inline for auto-expand.

use crate::State::AphroditeState;

/// Resolve a freshly stored hash through the crate's recursive resolution
/// entry point (port of the pre-Rust `_resolve_recursive` auto-expand path).
/// Call only when [`crate::Hook::auto_expand_applies`] is true. Resolution
/// failure degrades to the original content (warn + degrade house style, never
/// panic) - the payload still carries a valid marker for manual retrieval
/// either way, so a failed resolve costs the model a marker, not its content.
pub(crate) fn Fn(State:&mut AphroditeState, hash:&str, Content:&str) -> String {
	match crate::Resolve::Expand::Fn(State, hash) {
		Some(Expanded) => Expanded,
		None => {
			tracing::warn!(
				hash,
				"auto-expand: inline resolve failed for freshly stored hash; returning original content"
			);
			Content.to_string()
		},
	}
}
