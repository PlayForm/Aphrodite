//! Match a command against the known-slow table and mint its identity.
//!
//! `None` for an empty or absent command - a command the poll worker cannot see is
//! a command it must not background, since the resulting task could never be
//! matched back to a poll. The empty case needs no test of its own: an empty string
//! cannot start with a non-empty prefix.

use crate::PollWorker::Should::{Command::Prefix::Const::SlowCommandPrefixes, Plan::Struct::BackgroundPlan};

/// The identity and label for `command`, when it is a known-slow one.
pub(crate) fn Fn(command:Option<&str>) -> Option<BackgroundPlan> {
	let Command = command?.trim();
	let Lower = Command.to_lowercase();

	SlowCommandPrefixes.iter().find(|Prefix| Lower.starts_with(**Prefix))?;

	Some(BackgroundPlan {
		Identity:headroom_core::ccr::compute_key(Command.as_bytes()),
		Label:Command.chars().take(60).collect(),
	})
}
