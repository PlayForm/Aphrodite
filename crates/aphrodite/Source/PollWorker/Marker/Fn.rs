//! The poll-worker marker the agent sees in place of the raw output.
//!
//! The marker is deliberately typed `Poll`, not the content type of the
//! output: retrieving a `Poll` marker returns stale content, which is exactly
//! the mistake this marker exists to prevent. The hash it carries is the
//! task hash, not the output hash - output does not exist yet at creation
//! time, and the completion nudge supplies the output hash later.

/// Create a "poll worker" CCR marker that replaces the raw tool output.
/// The agent sees this marker instead of the original content and knows
/// to poll for progress rather than retrieve (which would return stale
/// content).
pub fn CreatePollMarker(TaskID:&str, CommandSummary:&str, Turn:usize) -> String {
	let Preview = format!("[poll:running {} | turn {}]", CommandSummary, Turn);
	let hash = TaskID;
	crate::Marker::Format::Entry::CCRMarker(hash, "poll", 0, &Preview, None, None, None)
}
