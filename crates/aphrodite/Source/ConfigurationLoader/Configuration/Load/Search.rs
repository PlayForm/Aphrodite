//! The default search: cwd first, then the runtime home.

use std::{collections::HashMap, path::PathBuf};

use super::super::Struct::Struct;
use crate::ConfigurationLoader::Warn::Fn::Fn as Warn;

impl Struct {
	/// Load from TOML file. Returns defaults on any failure.
	///
	/// Search order is `./aphrodite.toml` then `~/.hermes/aphrodite/aphrodite.toml`,
	/// never a path relative to the binary or the plugin directory. The first
	/// readable file that parses wins; a readable file that does NOT parse is
	/// warned about and skipped, so a broken local file cannot mask a valid
	/// one further down the search. Only when the search ends on defaults is
	/// the failure recorded on `ParseFailure`.
	pub fn Load() -> Self {
		let SearchPaths = [PathBuf::from("aphrodite.toml"), crate::Home::Configuration::Fn::Fn()];

		let mut ParseFailure:Option<String> = None;
		let Found = SearchPaths.iter().find_map(|path| {
			match std::fs::read_to_string(path) {
				Ok(Content) => {
					match Content.parse::<toml::Table>() {
						Ok(Table) => Some(Self { raw:Table, Overrides:HashMap::new(), ParseFailure:None }),
						Err(Error) => {
							// Fix 22 (inspection): a found-but-broken TOML file
							// used to fall through silently to the next search
							// path (or defaults) - warn so a broken local
							// aphrodite.toml is not ignored without indication.
							// `ParseFailure` is only recorded when the search
							// ends on defaults (a later path may still yield a
							// valid config).
							Warn(path, &Error, "skipping");

							ParseFailure = Some(format!("{}: {Error}", path.display()));
							None
						},
					}
				},
				// File not found/unreadable is the normal search miss - keep
				// looking.
				Err(_) => None,
			}
		});

		match Found {
			Some(Configuration) => Configuration,
			None => Self { raw:toml::Table::new(), Overrides:HashMap::new(), ParseFailure },
		}
	}
}
