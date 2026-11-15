//! The configuration struct itself.

use std::collections::HashMap;

/// Configuration value resolution: env var → TOML → default.
///
/// One `Struct` per loaded file. `raw` is the parsed TOML table, `Overrides`
/// is the runtime map that outranks both the environment and the file, and
/// `ParseFailure` records why a found-but-broken file was abandoned.
///
/// A `Struct` is never an error value: a missing file, an unreadable file and
/// a syntactically broken file all produce a usable `Struct` holding defaults.
/// The only difference a caller can observe is `ParseFailure` (and the
/// warning `Warn::Fn` emits), which is exactly what keeps "defaults in effect
/// because nothing is configured" distinguishable from "defaults in effect
/// because the file is broken".
pub struct Struct {
	/// The parsed TOML table. Empty when no file was found or the file failed
	/// to parse - an empty table resolves every knob to its default.
	pub(crate) raw:toml::Table,
	/// Runtime overrides set through `SetOverride`, keyed by env-var name.
	/// They outrank both the environment and the TOML file, which is what
	/// makes them usable as a test and operator escape hatch.
	pub(crate) Overrides:HashMap<String, String>,
	/// Set when a found `aphrodite.toml` failed to parse and the loader
	/// fell back to defaults - carried into `AphroditeState::config_error`
	/// so `AphroditeStats` can surface "defaults in effect (parse failed)"
	/// and the failure is never indistinguishable from "config not set".
	pub ParseFailure:Option<String>,
}
