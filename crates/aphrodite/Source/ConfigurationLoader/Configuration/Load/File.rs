//! Loading an explicitly named file, bypassing the search path.

use std::collections::HashMap;

use super::super::Struct::Struct;
use crate::ConfigurationLoader::Warn::Fn::WarnParseFailure;

impl Struct {
	/// Load from an explicit TOML file path, bypassing `load()`'s search
	/// paths - used by `AphroditeInit` (01-F4/F9) so the handle-based C ABI
	/// init path shares this type's parsing/section/env-override logic
	/// instead of hand-parsing four `[compression]` keys directly (which had
	/// silently drifted from `ApplyCompression`'s own key names and never
	/// honored env var overrides at all).
	///
	/// Unlike `load()` there is no next candidate to fall through to: a file
	/// that exists but does not parse degrades to defaults immediately, with
	/// the parse error warned about and recorded on `ParseFailure`.
	pub fn LoadFrom(path:&str) -> Self {
		match std::fs::read_to_string(path) {
			Ok(Content) => {
				match Content.parse::<toml::Table>() {
					Ok(Table) => Self { raw:Table, Overrides:HashMap::new(), ParseFailure:None },
					Err(Error) => {
						// Fix 22 (inspection): same warn-on-parse-error treatment
						// as `load()` - the explicit-path init used to silently
						// fall back to defaults on a broken file. `ParseFailure`
						// is recorded so `AphroditeStats` can self-diagnose.
						WarnParseFailure(std::path::Path::new(path), &Error, "using defaults");

						Self {
							raw:toml::Table::new(),
							Overrides:HashMap::new(),
							ParseFailure:Some(format!("{path}: {Error}")),
						}
					},
				}
			},
			Err(_) => Self::default(),
		}
	}
}
