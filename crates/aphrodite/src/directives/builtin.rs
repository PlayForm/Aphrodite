//! Built-in directives baked into the binary via `include_str!`.

use std::collections::HashMap;

use super::model::{Directive, MAX_DIRECTIVE_CHARS};

/// Built-in directives baked into the binary via `include_str!`.
/// These ship with every installation and are used as fallbacks when no
/// `directives/` directory exists on disk - so a fresh install gets
/// `focus`, `foresight`, `ccr-handling`, `cleanup`, `explore`, `lazy`,
/// and `lazy-eval` without any filesystem setup.
///
/// The on-disk `directives/*.md` files (if any) take precedence: if the
/// directory exists, its `.md` files replace these defaults entirely.
/// Users can also `aphrodite_directive("add", "ccr-handling")` to
/// activate the shipped defaults discovered from the embedded set.
fn builtin_directives() -> Vec<(&'static str, &'static str)> {
	vec![
		("focus", include_str!("../builtin_directives/focus.md")),
		("foresight", include_str!("../builtin_directives/foresight.md")),
		("ccr-handling", include_str!("../builtin_directives/ccr-handling.md")),
		("cleanup", include_str!("../builtin_directives/cleanup.md")),
		("explore", include_str!("../builtin_directives/explore.md")),
		("lazy", include_str!("../builtin_directives/lazy.md")),
		("lazy-eval", include_str!("../builtin_directives/lazy-eval.md")),
	]
}

/// Load built-in directives from the binary (via `include_str!`), applying the
/// same `MAX_DIRECTIVE_CHARS` cap as `load_directives` does for disk-loaded
/// directives. Returns a `HashMap` ready for `state.directives`.
pub fn loaded_builtins() -> HashMap<String, Directive> {
	let mut directives = HashMap::new();
	for (name, content) in builtin_directives() {
		let content = if content.len() > MAX_DIRECTIVE_CHARS {
			let trunc:String = content.chars().take(MAX_DIRECTIVE_CHARS).collect();
			format!("{}…", trunc)
		} else {
			content.to_string()
		};
		directives.insert(name.to_string(), Directive { name:name.to_string(), content });
	}
	directives
}
