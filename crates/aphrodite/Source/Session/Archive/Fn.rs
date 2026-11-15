//! Record one archived compression against the current turn.
//!
//! The conversation index is a map keyed by turn, bounded by `ConvIndexCap`:
//! exceeding the cap evicts the OLDEST turn, because the model's useful
//! history is the recent end. Evicting the newest would make every archive
//! immediately unreachable.

use crate::{Session::ConvIndexCap, State::AphroditeState};

/// Archive a compression at end of turn into the conversation index.
pub fn ArchiveTurn(State:&mut AphroditeState, hash:&str, Summary:&str, Size:usize) {
	let Turn = State.TurnCounter;
	State.ConvIndex.insert(Turn, (hash.to_string(), Summary.to_string(), Size));
	if State.ConvIndex.len() > ConvIndexCap {
		let Oldest = State.ConvIndex.keys().min().copied();
		if let Some(Key) = Oldest {
			State.ConvIndex.remove(&Key);
		}
	}
}
