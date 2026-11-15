//! Decide what the context engine will do, with no session mutation.

use crate::{
	Session::{
		Engine::{
			Assess::Enum::Enum as Assessment,
			Gate::{Enum::Enum as Gate, Resolve::Fn::Fn as ResolveGate},
			Plan::Build::Fn::Fn as BuildPlan,
		},
		Turn::Const::ConvIndexCap,
	},
	State::Struct::AphroditeState,
};

/// Assess the index and answer either a refusal or a fully built archive plan.
///
/// The metric (documented per the pre-Rust-port brief): the dylib never receives
/// the raw conversation message list (Hermes holds it) and has no token counts or
/// context length, so the closest available message-count proxy is the
/// conversation index itself - `ConvIndex.len()` is the number of archived turns,
/// and the "fill ratio" is `len * 100 / ConvIndexCap`. The index's own eviction
/// cap is the only "full" reference available.
///
/// Read-only: the gates and the packing both borrow the session, so a refusal and
/// a plan cost the same nothing, and the caller decides whether to apply.
pub(crate) fn Fn(State:&AphroditeState) -> Assessment {
	let Total = State.ConvIndex.len();
	// Fill ratio: archived-turn count against the index's own cap.
	let FillPct = ((Total as u64 * 100) / ConvIndexCap as u64).min(100);
	let HeadN = State.EngineProtectFirst;
	let TailN = State.EngineProtectLast;
	let MiddleN = Total.saturating_sub(HeadN).saturating_sub(TailN);

	match ResolveGate(State, Total, FillPct, MiddleN) {
		Some(Reason) => Assessment::Skip(Reason),
		None => {
			BuildPlan(State, Total, HeadN, TailN, MiddleN)
				.map_or(Assessment::Skip(Gate::Unpackable), Assessment::Archive)
		},
	}
}
