//! Record one archived compression against the current turn.

use crate::{Session::Turn::Const::ConvIndexCap, State::Struct::AphroditeState};

/// Archive a compression at end of turn into the conversation index.
///
/// The index is a map keyed by turn, bounded by `ConvIndexCap`: exceeding the cap
/// evicts the OLDEST turn, because the model's useful history is the recent end -
/// evicting the newest would make every archive immediately unreachable.
///
/// Eviction is one optional value: "the index is over its cap" resolves to "the
/// oldest key", and an absent key means there was nothing to evict.
pub fn ArchiveTurn(State:&mut AphroditeState, hash:&str, Summary:&str, Size:usize) {
	let Turn = State.TurnCounter;
	State.ConvIndex.insert(Turn, (hash.to_string(), Summary.to_string(), Size));

	let OverCap = State.ConvIndex.len() > ConvIndexCap;
	let Oldest = OverCap.then(|| State.ConvIndex.keys().min().copied()).flatten();

	Oldest.into_iter().for_each(|Key| {
		State.ConvIndex.remove(&Key);
	});
}
