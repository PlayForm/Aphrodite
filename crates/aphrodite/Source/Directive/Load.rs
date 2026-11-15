//! Disk loading of `directives/*.md` files into `Directive`s, with the
//! baked-in `include_str!` set as the missing-directory fallback.
//!
//! Issue #6 semantics: an existing directory that yields zero readable `.md`
//! files is an **intentionally empty** directive set - the returned map is
//! empty and NO built-in fallback is applied, so a caller can honor "I want
//! zero directives". Only when the directory itself is missing or unreadable
//! (`read_dir` fails) do the built-in directives come back as a fallback.
//! Getting this backwards makes "I turned all directives off" indistinguishable
//! from "the install is broken".

use std::{collections::HashMap, path::PathBuf};

use super::{
	Builtin::Fn as Builtins,
	Model::{MaximumDirectiveChars, Struct},
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
pub fn Fn(Directory:&PathBuf) -> HashMap<String, Struct> {
	let Entries = match std::fs::read_dir(Directory) {
		Ok(Entries) => Entries,
		Err(e) => {
			// Missing/unreadable directory: fall back to the baked-in set so a
			// fresh install (or a missing `~/.hermes/aphrodite/directives`)
			// still gets shipped defaults without any filesystem setup.
			tracing::warn!(
				DirectiveSource = "builtins",
				path = %Directory.display(),
				error = %e,
				"directives directory missing or unreadable; falling back to built-in directives"
			);
			return Builtins();
		},
	};

	let mut Directives = HashMap::new();

	Entries
		.flatten()
		.filter_map(|Entry| -> Option<(String, String)> {
			let path = Entry.path();

			match path.extension().map(|e| e != "md").unwrap_or(true) {
				true => None,
				false => {
					let name = path.file_stem().and_then(|n| n.to_str())?;
					let Content = std::fs::read_to_string(&path).ok()?;

					Some((name.to_string(), Content))
				},
			}
		})
		.for_each(|(name, Content)| {
			// Trim each directive to a reasonable size.
			let Content = match Content.len() > MaximumDirectiveChars {
				true => {
					let Trunc:String = Content.chars().take(MaximumDirectiveChars).collect();

					format!("{}...", Trunc)
				},
				false => Content,
			};

			Directives.insert(name.clone(), Struct { name, Content });
		});

	tracing::info!(
		DirectiveSource = "disk",
		path = %Directory.display(),
		Count = Directives.len(),
		"loaded {} directive(s) from disk",
		Directives.len()
	);

	match Directives.is_empty() {
		true => {
			// The directory exists but yields no readable `.md` files - this is an
			// *intentional* empty directive set, NOT a builtin-fallback trigger.
			tracing::info!(
				DirectiveSource = "disk",
				path = %Directory.display(),
				"directives directory exists but contains no readable .md files - treating as intentionally empty"
			);
		},
		false => (),
	}

	Directives
}
