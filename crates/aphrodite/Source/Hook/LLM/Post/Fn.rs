//! The post-LLM-call hook.

use crate::State::Struct::AphroditeState;

/// Post-LLM call hook - archive turn.
///
/// Archives the largest marker recorded this turn into `ConvIndex` before
/// advancing the turn counter (report 06 F11/T13/F13) - previously `ArchiveTurn`
/// was never called from any hook, so `ConvIndex` stayed empty forever and
/// `aphrodite_diff` always returned zero turns despite compressions
/// happening every turn.
pub fn Fn(State:&mut AphroditeState) -> serde_json::Value {
	let Span = tracing::info_span!(
		"hook.post_llm",
		turn = State.TurnCounter,
		archived = tracing::field::Empty,
		archived_size = tracing::field::Empty,
	);
	let _Entered = Span.enter();
	// Archive the LARGEST marker recorded this turn (T13/F13): the last
	// recorded marker is usually the least informative (e.g. a tiny confirm
	// echo), while the largest carries the most content for the conversation
	// index. Ties keep the later-recorded entry.
	let Largest = State
		.RecentMarkers
		.iter()
		.filter(|m| m.Turn == State.TurnCounter)
		.max_by_key(|m| m.Size);
	// Outcome first, while the borrow is still live: whether this turn had
	// anything worth indexing, and how big it was. The hash identifies the
	// entry without putting a payload near a log line.
	Span.record("archived", Largest.is_some());
	Span.record("archived_size", Largest.map(|m| m.Size).unwrap_or(0));
	match Largest {
		Some(Largest) => {
			let (hash, Summary, Size) = (Largest.hash.clone(), Largest.Preview.clone(), Largest.Size);
			crate::Session::Archive::Fn::ArchiveTurn(State, &hash, &Summary, Size);
		},
		None => {},
	}
	crate::Session::Turn::Fn::NextTurn(State);
	// P3/T9: purge expired ephemeral nudges AFTER advancing the turn, so a
	// one-shot pushed during turn N renders in turn N+1's context exactly once
	// and is gone by turn N+2.
	crate::Flow::Nudge::Purge::Fn::PurgeExpiredNudges(State);
	// Poll-worker: expire stale tasks (not polled in >STALE_TURN_AGE turns)
	// and drop completed/failed tasks older than 16 turns.
	crate::PollWorker::Expire::Fn::ExpireStaleTasks(State);
	serde_json::json!({"status": "ok", "turn": State.TurnCounter})
}
