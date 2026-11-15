//! Append a poll payload to a bounded accumulator.
//!
//! The room is computed once, the separator is charged to it, and the content is
//! cut to what is left - so the cap holds at every point instead of being
//! promised by a test that ran before the append. A separator with no room for
//! any content is not appended either: a trailing newline would be a byte spent
//! to say nothing.

use crate::PollWorker::Output::{Append::Enum::Append, Prefix::Fn::Fn as Prefix, Struct::Accumulator};

/// Append `Content`, reporting how much of it was kept.
pub(crate) fn Fn(This:&mut Accumulator, Content:&str) -> Append {
	// A separator is owed between two polls, and it is part of the buffer, so it
	// is charged to the same bound.
	let Separator = usize::from(!This.Buffer.is_empty());
	let Room = This.Maximum.saturating_sub(This.Buffer.len());
	let Available = Room.saturating_sub(Separator);

	match Prefix(Content, Available) {
		None => Append::Full,
		Some(Kept) => {
			This.Buffer.extend(std::iter::repeat_n('\n', Separator));
			This.Buffer.push_str(Kept);

			match Content.get(Kept.len()..).filter(|Rest| !Rest.is_empty()) {
				None => Append::Complete,
				Some(_) => Append::Truncated,
			}
		},
	}
}
