//! Record one archived compression against the current turn.
//!
//! The conversation index is a map keyed by turn, bounded by `ConvIndexCap`:
//! exceeding the cap evicts the OLDEST turn, because the model's useful
//! history is the recent end. Evicting the newest would make every archive
//! immediately unreachable.

use crate::{Session::Turn::Fn::ConvIndexCap, State::Struct::AphroditeState};

/// Archive a compression at end of turn into the conversation index.
pub fn ArchiveTurn(State:&mut AphroditeState, hash:&str, Summary:&str, Size:usize) {
	let Turn = State.TurnCounter;
	State.ConvIndex.insert(Turn, (hash.to_string(), Summary.to_string(), Size));

	match State.ConvIndex.len() > ConvIndexCap {
		true => {
			let Oldest = State.ConvIndex.keys().min().copied();

			match Oldest {
				Some(Key) => {
					State.ConvIndex.remove(&Key);
				},
				None => (),
			}
		},
		false => (),
	}
}
