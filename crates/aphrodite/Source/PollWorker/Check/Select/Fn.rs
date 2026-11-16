//! Which tasks speak this turn.
//!
//! Eligibility is "not stale": a stale task is one the agent already walked away
//! from, so a nudge about it spends the turn's budget on nothing. The ceiling is on
//! the nudges *emitted* - `take(2)` after the eligibility filter, never before it,
//! or a list of stale tasks would consume the budget and emit nothing.
//!
//! The walk is newest-first. That is the order the old loop actually executed
//! (`(0..len).rev()`); its comment said "oldest first" and was wrong, and this
//! rewrite corrects the comment rather than the policy.

use crate::PollWorker::{
	Registry::{IterateRev::Fn::Fn as IterateRev, Reader::Struct::Reader},
	Task::{Status::Enum::Enum as Status, Struct::Struct as Record},
};

/// At most two tasks, newest first, stale ones silent.
pub(crate) fn Fn<'a>(This:&'a Reader<'_>) -> Vec<&'a Record> {
	IterateRev(This).filter(|Task| Task.Status != Status::Stale).take(2).collect()
}
