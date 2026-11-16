//! Store a completed poll's output and record its marker.
//!
//! The output is moved into the store, never copied: `Completion::Build` takes the
//! applied value by value, so the buffer the poll moved out of the task is the buffer
//! the store receives. The task then records the hash, preview and size and takes the
//! status its exit code implies.
//!
//! Order is behaviour. The payload is stored before the marker that names it, and the
//! marker before the task's status: an interruption leaves content with no marker, or a
//! marker with a task that has not yet been marked finished - never a finished task whose
//! content is missing. A store entry evicted by the byte budget immediately after the put
//! is warned about rather than silently announced, because a marker that does not resolve
//! is indistinguishable from a retrieve bug.

use crate::{
	PollWorker::{
		Completion::Struct::Completion,
		Registry::{Complete::Fn::Fn as Complete, From::Fn::Fn as Registry},
	},
	State::{Marker::Struct::Struct as Entry, Struct::AphroditeState},
};

/// Put the output in the store, record the marker, complete the task.
pub(crate) fn Fn(State:&mut AphroditeState, Index:usize, Completion:Completion) {
	let Completion { Stored, Output, Command, ExitCode } = Completion;

	State.Put(Stored.Hash.as_str().to_string(), Output);

	// The store decides whether the completion is still there: an entry the budget
	// evicted immediately is a listed hash that cannot resolve, so the size it lost
	// is what the warning reports.
	(!State.Contains(Stored.Hash.as_str()))
		.then_some(Stored.Size)
		.into_iter()
		.for_each(|Size| {
			tracing::warn!(
				Hash = %Stored.Hash,
				Size = Size,
				"poll worker: completion was evicted by the inline store budget immediately after it was stored"
			);
		});

	State.RecordMarker(Entry {
		hash:Stored.Hash.as_str().to_string(),
		CCRType:"terminal".to_string(),
		Size:Stored.Size,
		Preview:Stored.Preview.clone(),
		Turn:State.TurnCounter,
		Center:Some(format!("poll:{Command}")),
		meta:None,
	});

	match Complete(&mut Registry(&mut State.BgTasks), Index, Stored, ExitCode) {
		Some(()) => (),
		// The task was admitted when the poll applied to it, so this is the state a
		// panic between the two phases would leave - the output and its marker are
		// real, the task record is gone, and the expiry pass is what would have
		// removed it. Say so instead of pretending the completion landed on a task.
		None => {
			tracing::warn!(Index, "poll worker: completion stored for a task that is no longer tracked")
		},
	}
}
