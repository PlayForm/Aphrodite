//! Admit one task, at most one eviction, never over capacity.
//!
//! Exactly one removal, not a repair loop: every insertion in the tree goes through
//! this function and the bound is a non-zero constant, so the registry is never
//! over capacity on entry and one eviction is always enough. The loop it replaces
//! existed only to repair a vector someone else had already overfilled.

use crate::PollWorker::{
	Registry::{
		Admit::Enum::Admit,
		Read::Fn::Fn as Read,
		Room::{Enum::Room, Fn::Fn as RoomOf},
		Struct::Registry,
	},
	Task::Struct::Struct as Record,
};

/// Admit `Task`, evicting one expendable task when the set is full.
pub(crate) fn Fn(This:&mut Registry, Task:Record) -> Admit {
	match RoomOf(&Read(This), &Task.ID, This.Bound) {
		Room::Duplicate => Admit::Duplicate,
		Room::Free => {
			This.Tasks.push_back(Task);
			Admit::Admitted
		},
		Room::Evict(Some(Index)) => {
			let Gone = This.Tasks.remove(Index).map(|Gone| Gone.command).unwrap_or_default();
			This.Tasks.push_back(Task);
			Admit::Evicted { Label:Gone }
		},
		// A registry at capacity is never empty, so `Evict` always names a victim.
		// This arm exists because admission must stay total, not because the state
		// is reachable.
		Room::Evict(None) => {
			This.Tasks.push_back(Task);
			Admit::Admitted
		},
	}
}
