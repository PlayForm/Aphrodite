//! Built-in directives baked into the binary via `include_str!`.
//!
//! These ship with every installation and are the fallback when no
//! `directives/` directory exists on disk - a fresh install gets `focus`,
//! `foresight`, `ccr-handling`, `cleanup`, `explore`, `lazy` and `lazy-eval`
//! without any filesystem setup. The filenames under `BuiltinDirective/` are
//! the directive identifiers surfaced to the model, so they stay lowercase.

use std::collections::HashMap;

use super::Model::{MaximumDirectiveChars, Struct};

/// The embedded directive set: identifier and body, in the order they are
/// advertised.
fn BuiltinDirectives() -> Vec<(&'static str, &'static str)> {
	vec![
		("focus", include_str!("../BuiltinDirective/focus.md")),
		("foresight", include_str!("../BuiltinDirective/foresight.md")),
		("ccr-handling", include_str!("../BuiltinDirective/ccr-handling.md")),
		("cleanup", include_str!("../BuiltinDirective/cleanup.md")),
		("explore", include_str!("../BuiltinDirective/explore.md")),
		("lazy", include_str!("../BuiltinDirective/lazy.md")),
		("lazy-eval", include_str!("../BuiltinDirective/lazy-eval.md")),
	]
}

/// Load built-in directives from the binary (via `include_str!`), applying the
/// same `MaximumDirectiveChars` cap as `LoadDirectives` does for disk-loaded
/// directives. Returns a `HashMap` ready for `state.directives`.
///
/// The on-disk `directives/*.md` files (if any) take precedence: if the
/// directory exists, its `.md` files replace these defaults entirely. Users
/// can also `aphrodite_directive("add", "ccr-handling")` to activate the
/// shipped defaults discovered from the embedded set.
pub fn Fn() -> HashMap<String, Struct> {
	let mut Directives = HashMap::new();
	for (name, Content) in BuiltinDirectives() {
		let Content = if Content.len() > MaximumDirectiveChars {
			let Trunc:String = Content.chars().take(MaximumDirectiveChars).collect();
			format!("{}...", Trunc)
		} else {
			Content.to_string()
		};
		Directives.insert(name.to_string(), Struct { name:name.to_string(), Content });
	}
	Directives
}
