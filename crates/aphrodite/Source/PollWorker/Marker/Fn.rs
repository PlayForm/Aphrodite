//! The poll-worker marker the agent sees in place of the raw output.
//!
//! The marker is deliberately typed `Poll`, not the content type of the
//! output: retrieving a `Poll` marker returns stale content, which is exactly
//! the mistake this marker exists to prevent. The hash it carries is the
//! task hash, not the output hash - output does not exist yet at creation
//! time, and the completion nudge supplies the output hash later. Both are
//! identities of different kinds, so both are typed.

use crate::State::Identity::Command::Struct::Struct as Command;

/// Create a "poll worker" CCR marker that replaces the raw tool output.
/// The agent sees this marker instead of the original content and knows
/// to poll for progress rather than retrieve (which would return stale
/// content).
///
/// The text it returns is the external contract - `<<<CCR:<hash>|poll|0>>>`
/// with the command summary in the preview - because the model reads that
/// string, not the type. The type word is `poll` on purpose (retrieving a
/// `poll` marker answers with stale content) and the size is `0` because no
/// output exists yet at creation time.
pub fn Fn(TaskID:&Command, CommandSummary:&str, Turn:usize) -> String {
	let Preview = format!("[poll:running {} | turn {}]", CommandSummary, Turn);
	let hash = TaskID.as_str();
	crate::Marker::Format::Entry::Fn(hash, "poll", 0, &Preview, None, None, None)
}
