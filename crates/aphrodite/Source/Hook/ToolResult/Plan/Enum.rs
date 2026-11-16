//! The plan a tool-result transform selected.

use crate::Hook::{Reason::Enum::Enum as Reason, ToolResult::Input::Struct::Struct as Input};

/// What the tool-result path will do, decided before anything is executed.
///
/// Three plans, and the variants are the whole decision: nothing downstream
/// re-tests a threshold, a flag or the tool's name, so the policy cannot be read
/// two ways. There is no `Split` here - chain splitting is a terminal-output
/// concern, because only a shell command can carry segment markers.
///
/// Each variant carries the operands it executes: `Live` and `Compress` own the
/// `Input` they transform, so a plan selected for one tool result cannot be
/// executed against another, and `Compress` carries the resolved `AutoExpand`
/// decision rather than leaving the executor to read it out of a policy.
pub enum Enum<'a> {
	/// Return the content untouched, for the stated reason.
	Passthrough(Reason),
	/// Store this content as a live container and emit an instant marker.
	Live { Input:Input<'a> },
	/// Store this content and emit one ordinary marker.
	Compress { Input:Input<'a>, AutoExpand:bool },
}
