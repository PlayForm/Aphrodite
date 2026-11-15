//! Store a completed poll's output and record its marker.
//!
//! The output is moved into the store, never copied: `Completion::Build` takes the
//! applied value by value, so the buffer the poll moved out of the task is the
//! buffer the store receives. The task then records the hash and preview and takes
//! the status its exit code implies.

use crate::{
	PollWorker::{
		Completion::Struct::Completion,
		Registry::{Complete::Fn::Fn as Complete, From::Fn::Fn as Registry},
	},
	State::{Marker::Struct::MarkerEntry, Struct::AphroditeState},
};

/// Put the output in the store, record the marker, complete the task.
pub(crate) fn Fn(State:&mut AphroditeState, Index:usize, Completion:Completion) {
	State.InlineStorePut(Completion.Hash.clone(), Completion.Output);

	State.RecordMarker(MarkerEntry {
		hash:Completion.Hash.clone(),
		CCRType:"terminal".to_string(),
		Size:Completion.Size,
		Preview:Completion.Preview.clone(),
		Turn:State.TurnCounter,
		Center:Some(format!("poll:{}", Completion.Command)),
		meta:None,
	});

	Complete(
		&mut Registry(&mut State.BgTasks),
		Index,
		&Completion.Hash,
		&Completion.Preview,
		Completion.Size,
		Completion.ExitCode,
	);
}
