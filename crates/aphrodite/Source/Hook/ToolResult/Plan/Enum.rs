//! The plan a tool-result transform selected.

use crate::Hook::Reason::Enum::Enum as Reason;

/// What the tool-result path will do, decided before anything is executed.
///
/// Three plans, and the variants are the whole decision: nothing downstream
/// re-tests a threshold, a flag or the tool's name, so the policy cannot be read
/// two ways. There is no `Split` here - chain splitting is a terminal-output
/// concern, because only a shell command can carry segment markers.
pub enum Enum {
	/// Return the content untouched, for the stated reason.
	Passthrough(Reason),
	/// Store the content as a live container and emit an instant marker.
	Live,
	/// Store the content and emit one ordinary marker.
	Compress,
}
