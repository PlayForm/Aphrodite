//! The session's turn bookkeeping.
//!
//! Split out of `AphroditeState` so the turn-scoped readers and writers take
//! `&mut Turn::Struct` or `&Turn::Struct`: incrementing the counter, scanning
//! the message list incrementally and archiving a turn into the conversation
//! index are one domain, and none of them needs the store, the rings or the
//! configuration.

use std::collections::HashMap;

/// The turn counter, the incremental-scan cursor, and the per-turn conversation
/// index the context engine compresses.
#[derive(Clone)]
pub struct Struct {
	/// Turn counter.
	pub TurnCounter:usize,
	/// Scanned message index for incremental marker scan.
	pub ScannedMessageIndex:usize,
	/// Conversation index: {turn_num: (hash, summary, size)} - the last
	/// marker archived per turn by `Session::archive_turn`, called from
	/// `Hook::LLM::Post::Fn::Fn` (report 06 F11/T13: previously `Archive`
	/// was never called from any hook, so this stayed empty forever and
	/// `aphrodite_diff` always returned zero turns).
	pub ConvIndex:HashMap<usize, (String, String, usize)>,
}
