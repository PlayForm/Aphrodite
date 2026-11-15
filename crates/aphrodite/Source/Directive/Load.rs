//! Disk loading of `directives/*.md` files into directives.

use std::{collections::HashMap, path::PathBuf};

use super::{Builtin::Fn as Builtins, Cap, Entry, Model::Struct};

/// Load all `.md` files from a `directives/` directory.
///
/// Returns a map of name → directive. Files without a `.md` extension are silently
/// skipped.
///
/// Issue #6 semantics, which is the whole reason this is not a plain `read_dir` walk: an
/// existing directory that yields zero readable `.md` files is an **intentionally empty**
/// directive set - the returned map is empty and NO built-in fallback is applied, so a
/// caller can honour "I want zero directives". Only when the directory itself is missing
/// or unreadable (`read_dir` fails) do the built-in directives (baked into the binary via
/// `include_str!`) come back as a fallback, so a fresh install without a `directives/`
/// directory still gets `focus`, `foresight`, `ccr-handling`, `cleanup`, `explore`, `lazy`
/// and `lazy-eval`. Getting this backwards makes "I turned all directives off"
/// indistinguishable from "the install is broken".
pub fn Fn(Directory:&PathBuf) -> HashMap<String, Struct> {
	let Entries = match std::fs::read_dir(Directory) {
		Ok(Entries) => Entries,
		Err(Error) => {
			// Missing/unreadable directory: fall back to the baked-in set so a fresh
			// install (or a missing `~/.hermes/aphrodite/directives`) still gets the
			// shipped defaults without any filesystem setup.
			tracing::warn!(
				DirectiveSource = "builtins",
				path = %Directory.display(),
				error = %Error,
				"directives directory missing or unreadable; falling back to built-in directives"
			);

			return Builtins();
		},
	};

	let Directives:HashMap<String, Struct> = Entries
		.flatten()
		.filter_map(Entry::Fn::Fn)
		.map(|(name, Content)| (name.clone(), Struct { name, Content:Cap::Fn::Fn(&Content).into_owned() }))
		.collect();

	tracing::info!(
		DirectiveSource = "disk",
		path = %Directory.display(),
		Count = Directives.len(),
		"loaded {} directive(s) from disk",
		Directives.len()
	);

	// The directory exists but yields no readable `.md` files - an intentionally empty
	// set, NOT a builtin-fallback trigger. Logged so the distinction is observable rather
	// than inferred from an empty map.
	let _ = Directives.is_empty().then(|| {
		tracing::info!(
			DirectiveSource = "disk",
			path = %Directory.display(),
			"directives directory exists but contains no readable .md files - treating as intentionally empty"
		);
	});

	Directives
}
