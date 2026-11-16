//! Record one archived compression against the current turn.

use crate::{Session::Turn::Const::Cap, State::Turn::Struct::Struct as Turn};

/// Archive a compression at end of turn into the conversation index.
///
/// Takes the turn bookkeeping rather than the session state: this step writes
/// one map and reads one counter, so it has no business reaching the store, the
/// rings or the configuration.
///
/// The index is a map keyed by turn, bounded by `Cap`: exceeding the cap
/// evicts the OLDEST turn, because the model's useful history is the recent end -
/// evicting the newest would make every archive immediately unreachable.
///
/// Eviction is one optional value: "the index is over its cap" resolves to
/// "the oldest key other than the one just written", and an absent key means
/// there was nothing to evict.
///
/// The exclusion is not defensive noise: a turn counter that went backwards
/// makes the new entry the OLDEST key, and evicting that would delete the
/// archive this call exists to record while keeping the entries it should have
/// displaced.
pub fn Fn(Turn:&mut Turn, hash:&str, Summary:&str, Size:usize) {
	let Now = Turn.Counter;
	Turn.Index.insert(Now, (hash.to_string(), Summary.to_string(), Size));

	let OverCap = Turn.Index.len() > Cap;
	let Oldest = OverCap
		.then(|| Turn.Index.keys().copied().filter(|Key| *Key != Now).min())
		.flatten();

	Oldest.into_iter().for_each(|Key| {
		Turn.Index.remove(&Key);
	});
}
