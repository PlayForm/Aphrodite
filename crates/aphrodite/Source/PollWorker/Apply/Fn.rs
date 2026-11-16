//! Select the task a poll belongs to, then apply it there.
//!
//! One task, always: the old ingestion applied a single exit code to every running
//! task, so the first poll carrying one finished a second, still-running build and
//! recorded the other task's code on it (audit D6). `Select::Target` owns the
//! identification rule, `Registry::Poll` owns the mutation.

use crate::PollWorker::{
	Apply::Applied::Enum::Enum as Outcome,
	Event::Struct::Struct as Wire,
	Registry::{Poll::Fn::Fn as Poll, Read::Fn::Fn as Read, Struct::Registry},
	Select::Target::Fn::Fn as Target,
};

/// Apply `Event` to the one task it belongs to, if any.
///
/// `None` is the answer for a poll that names no tracked running task - a poll for a
/// process this session is not tracking. It is not "ignored" and not "progressed":
/// the result is simply not ours, so the caller passes it through as ordinary output.
pub(crate) fn Fn(This:&mut Registry, Event:&Wire<'_>, Turn:usize) -> Option<Outcome> {
	let Index = Target(&Read(This), Event)?;

	Poll(This, Index, Event, Turn)
}
