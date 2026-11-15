//! Select the task a poll belongs to, then apply it there.
//!
//! One task, always: the old ingestion applied a single exit code to every running
//! task, so the first poll carrying one finished a second, still-running build and
//! recorded the other task's code on it (audit D6). `Select::Target` owns the
//! identification rule, `Registry::Poll` owns the mutation.

use crate::PollWorker::{
	Apply::Applied::Struct::Applied,
	Event::Struct::PollEvent,
	Registry::{Poll::Fn::Fn as Poll, Read::Fn::Fn as Read, Struct::Registry},
	Select::Target::Fn::Fn as Target,
};

/// Apply `Event` to the one task it belongs to, if any.
pub(crate) fn Fn(This:&mut Registry, Event:&PollEvent, Turn:usize) -> Option<Applied> {
	let Index = Target(&Read(This), Event)?;

	Poll(This, Index, Event, Turn)
}
