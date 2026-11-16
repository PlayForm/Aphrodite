//! Built-in directives baked into the binary via `include_str!`.

use std::collections::HashMap;

use super::{Cap, Embedded, Model::Struct};

/// Load the built-in directives from the binary, applying the same per-file cap the disk
/// loader applies. Returns a map ready for `state.Directive.Loaded`.
///
/// These ship with every installation and are the fallback when no `directives/`
/// directory exists on disk - a fresh install gets `focus`, `foresight`, `ccr-handling`,
/// `cleanup`, `explore`, `lazy` and `lazy-eval` without any filesystem setup.
///
/// The on-disk `directives/*.md` files (if any) take precedence: if the directory exists,
/// its `.md` files replace these defaults entirely. Users can also
/// `aphrodite_directive("add", "ccr-handling")` to activate a shipped default discovered
/// from this set.
///
/// The cap goes through the same [`Cap::Fn::Fn`] as the disk loader, so a shipped
/// directive and a user's copy of it are cut by the same rule at the same place.
pub fn Fn() -> HashMap<String, Struct> {
	Embedded::Const::Const
		.iter()
		.map(|(name, Content)| {
			(
				(*name).to_string(),
				Struct { name:(*name).to_string(), Content:Cap::Fn::Fn(Content).into_owned() },
			)
		})
		.collect()
}
