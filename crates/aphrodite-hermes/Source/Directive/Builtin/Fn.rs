//! The set of directives provisioning writes: the core crate's embedded
//! builtins, sorted.
//!
//! The dylib owns no directive text of its own - the `.md` sources are
//! compiled into the core `aphrodite` crate - so this module is a thin,
//! deterministic projection of `aphrodite::Directive::Builtin::Fn()`.
//! Sorting here (rather than at the write site) is what makes the report's
//! `written`/`skipped` arrays stable across runs.

use crate::Directive::Entry;

/// The set to provision: the core crate's embedded builtins.
///
/// `loaded_builtins()` caps content at `MAX_DIRECTIVE_CHARS` (2000); every
/// shipped file is currently well under the cap, so the materialized bytes
/// match the embedded sources verbatim. Sorted by name for deterministic
/// output ordering.
///
/// It is not a loader: nothing is read from disk, so a user-modified file in
/// the runtime home never appears here (that difference is exactly what
/// `Materialize` reports as `skipped`).
pub(crate) fn Fn() -> Vec<Entry::Struct::Struct> {
	let mut Set:Vec<Entry::Struct::Struct> = aphrodite::Directive::Builtin::Fn()
		.into_iter()
		.map(|(Name, Directive)| Entry::Struct::Struct { Name, Content:Directive.Content })
		.collect();

	Set.sort_by(|Left, Right| Left.Name.cmp(&Right.Name));

	Set
}
