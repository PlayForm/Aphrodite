//! The variables derived from the rest of the set.

use super::Struct::Struct;

/// Derive `cmx` from the command line.
///
/// Runs LAST, because it reads a value an earlier pass wrote. `fx` is not derived
/// here - it is never derivable from content alone, so it is one of the flat defaults
/// rather than a computation that always answers the same thing.
///
/// An absent command renders `"?"` rather than an empty cell: a template printing
/// `{cmx}` must show that there was no command, which is what the pre-Rust engine did.
pub(crate) fn Fn(Command:&str) -> Struct {
	Struct {
		Cmx:Some(Command)
			.filter(|Command| !Command.is_empty())
			.map_or_else(|| "?".to_string(), |Command| Command.chars().take(40).collect()),
	}
}
