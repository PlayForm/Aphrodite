//! Session lifecycle: turn archiving and the poll-worker checkpoints.

use crate::Hook::LLM::Post::Fn::Fn as PostLLMCall;
use crate::Hook::LLM::Pre::Fn::Fn as PreLLMCall;
use crate::State::Struct::AphroditeState;
use crate::State::Marker::Struct::MarkerEntry;

// ── T13 (F11): post_llm_call must archive the turn's last marker into
// conv_index so aphrodite_diff returns non-zero results.
#[test]
fn TestPostLLMCallArchivesTurn() {
	let mut s = AphroditeState::default();
	s.TurnCounter = 2;
	// Record a marker on the current turn.
	s.RecordMarker(MarkerEntry {
		hash:"hash42".into(),
		CCRType:"text".into(),
		Size:100,
		Preview:"[text] summary".into(),
		Turn:2,
		Center:None,
		meta:None,
	});
	let _ = PostLLMCall(&mut s);
	assert_eq!(s.TurnCounter, 3);
	assert_eq!(s.ConvIndex.len(), 1, "post_llm_call must archive the turn");
	assert_eq!(s.ConvIndex[&2].0, "hash42");
}

// ── T13 (F13): when a turn records several markers, post_llm_call must
// archive the LARGEST one, not merely the last recorded - the reverse
// find picked the least informative marker (e.g. a tiny confirm echo
// recorded after a big tool dump).
#[test]
fn TestPostLLMCallArchivesLargestSameTurnMarker() {
	let mut s = AphroditeState::default();
	s.TurnCounter = 7;
	let Record = |s:&mut AphroditeState, hash:&str, Size:usize| {
		s.RecordMarker(MarkerEntry {
			hash:hash.into(),
			CCRType:"text".into(),
			Size,
			Preview:format!("[text] {}", Size),
			Turn:7,
			Center:None,
			meta:None,
		});
	};
	Record(&mut s, "large", 900);
	// Two smaller markers recorded AFTER the large one - the old
	// reverse-find would have picked one of these.
	Record(&mut s, "small-1", 40);
	Record(&mut s, "small-2", 41);
	let _ = PostLLMCall(&mut s);
	assert_eq!(s.TurnCounter, 8);
	assert_eq!(s.ConvIndex.len(), 1, "post_llm_call must archive the turn");
	assert_eq!(s.ConvIndex[&7].0, "large", "largest same-turn marker must win");
}

// ── Poll-worker integration tests ──────────────────────────
// Agent-agnostic tests: check_bg_tasks in pre_llm_call and
// expire_stale_tasks in post_llm_call.  Agent-specific trigger
// tests (auto-backgrounding from tool results) live in
// aphrodite-hermes.

#[test]
fn TestPreLLMCallPushesPollNudges() {
	let mut s = AphroditeState::default();
	s.TurnCounter = 3;
	crate::PollWorker::Insert::Fn::InsertBgTask(&mut s, "task1".into(), "terminal".into(), "cargo build".into(), 1);
	let _ = PreLLMCall(&mut s);
	// The nudge should have been pushed into ephemeral_directives.
	assert!(!s.EphemeralDirectives.is_empty(), "pre_llm_call should push poll-worker nudge");
	let Nudge = s.EphemeralDirectives.last().unwrap();
	assert!(
		Nudge.inline.as_deref().unwrap().contains("cargo build"),
		"nudge should mention the command"
	);
}

#[test]
fn TestPostLLMCallExpiresStalePollTasks() {
	let mut s = AphroditeState::default();
	s.TurnCounter = 20;
	crate::PollWorker::Insert::Fn::InsertBgTask(
		&mut s,
		"old-task".into(),
		"terminal".into(),
		"old-build".into(),
		10, // started at turn 10, gap=10 < 16 so retain won't drop it
	);
	s.BgTasks[0].LastPollTurn = 10; // never polled after start, gap=10 > 8
	let _ = PostLLMCall(&mut s);
	// Should be marked stale.
	assert_eq!(
		s.BgTasks[0].Status,
		crate::PollWorker::Task::Status::Enum::BgStatus::Stale,
		"unpolled task should be expired as stale after {} turns",
		crate::PollWorker::Task::Const::StaleTurnAge
	);
}

#[test]
fn TestPreLLMCallSkipsNudgesWhenPollWorkerDisabled() {
	let mut s = AphroditeState::default();
	s.PollWorkerEnabled = false;
	s.TurnCounter = 3;
	crate::PollWorker::Insert::Fn::InsertBgTask(&mut s, "task1".into(), "terminal".into(), "cargo build".into(), 1);
	let Before = s.EphemeralDirectives.len();
	let _ = PreLLMCall(&mut s);
	assert_eq!(
		s.EphemeralDirectives.len(),
		Before,
		"disabled flag must prevent poll-worker nudges in pre_llm_call"
	);
}

#[test]
fn TestPostLLMCallStillExpiresWhenPollWorkerDisabled() {
	let mut s = AphroditeState::default();
	s.PollWorkerEnabled = false;
	s.TurnCounter = 20;
	crate::PollWorker::Insert::Fn::InsertBgTask(&mut s, "old-task".into(), "terminal".into(), "old-build".into(), 10);
	s.BgTasks[0].LastPollTurn = 10;
	let _ = PostLLMCall(&mut s);
	// Expiry is a cleanup concern - should still run even when disabled.
	assert_eq!(
		s.BgTasks[0].Status,
		crate::PollWorker::Task::Status::Enum::BgStatus::Stale,
		"expire_stale_tasks must still run when disabled (cleanup)"
	);
}
