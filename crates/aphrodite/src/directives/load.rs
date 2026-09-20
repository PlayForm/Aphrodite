//! Disk loading of `directives/*.md` files into `Directive`s, with the
//! baked-in `include_str!` set as the missing-directory fallback.

use std::{collections::HashMap, path::PathBuf};

use super::{
	builtin::loaded_builtins,
	model::{Directive, MAX_DIRECTIVE_CHARS},
};

/// Load all `.md` files from a `directives/` directory.
/// Returns a map of name → Directive. Files without `.md` extension are
/// silently skipped.
///
/// Issue #6 semantics: an existing directory that yields zero readable `.md`
/// files is an **intentionally empty** directive set - the returned map is
/// empty and NO built-in fallback is applied, so a caller can honor "I want
/// zero directives". Only when the directory itself is missing or unreadable
/// (`read_dir` fails) do the built-in directives (baked into the binary via
/// `include_str!`) come back as a fallback, so a fresh install without a
/// `directives/` directory still gets `focus`, `foresight`, `ccr-handling`,
/// `cleanup`, `explore`, `lazy`, and `lazy-eval`.
pub fn load_directives(dir:&PathBuf) -> HashMap<String, Directive> {
	let entries = match std::fs::read_dir(dir) {
		Ok(entries) => entries,
		Err(e) => {
			// Missing/unreadable directory: fall back to the baked-in set so a
			// fresh install (or a missing `~/.hermes/aphrodite/directives`)
			// still gets shipped defaults without any filesystem setup.
			tracing::warn!(
				directive_source = "builtins",
				path = %dir.display(),
				error = %e,
				"directives directory missing or unreadable; falling back to built-in directives"
			);
			return loaded_builtins();
		},
	};

	let mut directives = HashMap::new();
	for entry in entries.flatten() {
		let path = entry.path();
		if path.extension().map(|e| e != "md").unwrap_or(true) {
			continue;
		}
		let Some(name) = path.file_stem().and_then(|n| n.to_str()) else {
			continue;
		};
		let Ok(content) = std::fs::read_to_string(&path) else {
			continue;
		};
		// Trim each directive to a reasonable size.
		let content = if content.len() > MAX_DIRECTIVE_CHARS {
			let trunc:String = content.chars().take(MAX_DIRECTIVE_CHARS).collect();
			format!("{}…", trunc)
		} else {
			content
		};
		directives.insert(name.to_string(), Directive { name:name.to_string(), content });
	}
	tracing::info!(
		directive_source = "disk",
		path = %dir.display(),
		count = directives.len(),
		"loaded {} directive(s) from disk",
		directives.len()
	);
	if directives.is_empty() {
		// The directory exists but yields no readable `.md` files - this is an
		// *intentional* empty directive set, NOT a builtin-fallback trigger.
		tracing::info!(
			directive_source = "disk",
			path = %dir.display(),
			"directives directory exists but contains no readable .md files - treating as intentionally empty"
		);
	}
	directives
}
