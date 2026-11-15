//! DEV-ONLY pre-turn debug line.
//!
//! Emitted only while the debug logger is on, and only for the sections that
//! actually survived the budget - logging the candidates would report a turn
//! the model never saw. Nothing here contributes a byte to the injected
//! context; it is instrumentation, and the `T0` option is how the whole block
//! costs one atomic load when the logger is off.

use crate::{Flow::SectionLabel, State::AphroditeState};

/// Emit the `pre_turn_sections` line: one label + byte size per surviving
/// section, the flow budget, the floor, and the cumulative metrics snapshot.
pub(crate) fn LogPreTurn(
	State:&AphroditeState,
	T0:Option<std::time::Instant>,
	Budget:usize,
	AlwaysSurvive:usize,
	Sections:&[String],
) {
	if let Some(T0) = T0 {
		let Us = T0.elapsed().as_micros() as u64;
		crate::Debug::Timing::Add::Fn::Fn(crate::Debug::Timing::Enum::Enum::PreTurn, Us);
		let Secs:Vec<serde_json::Value> = Sections
			.iter()
			.map(|s| serde_json::json!({"label": SectionLabel(s), "bytes": s.len()}))
			.collect();
		crate::Debug::Log::At::Fn::Fn(
			State.DebugLevel,
			"pre_turn_sections",
			&[
				("turn", State.TurnCounter.to_string()),
				("budget", Budget.to_string()),
				("always_survive", AlwaysSurvive.to_string()),
				("sections", serde_json::to_string(&Secs).unwrap_or_default()),
				("pending_markers", State.RecentMarkers.len().to_string()),
				("metrics", crate::Debug::Metrics::Fn::Fn().to_string()),
				("elapsed_us", Us.to_string()),
			],
		);
	}
}
