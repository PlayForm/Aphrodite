//! `MetricsJSON`: the counters and timing averages as JSON.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::Debug::{
	Count::{Const::Counters, Enum::Enum as Counter},
	Timing::{
		Const::{TimingN, TimingSum},
		Enum::Enum as TimingCat,
	},
};

/// Snapshot the cumulative counters + timing averages as JSON (emitted on
/// each pre-turn line and surfaced in `AphroditeStats`).
///
/// Read-only and lock-free: every value is one relaxed load, so the snapshot is
/// a smear across a few microseconds rather than an instant. That is acceptable
/// for a counter report and is the reason it is never used as a correctness
/// check - only as telemetry.
pub fn Fn() -> serde_json::Value {
	let c = |Index:usize| Counters[Index].load(Ordering::Relaxed);
	let Avg = |Sum:&AtomicU64, n:&AtomicU64| -> f64 {
		let n = n.load(Ordering::Relaxed);
		match n == 0 {
			true => 0.0,
			false => Sum.load(Ordering::Relaxed) as f64 / n as f64,
		}
	};
	serde_json::json!({
		"markers_created": c(Counter::MarkersCreated as usize),
		"compressions": c(Counter::Compressions as usize),
		"auto_expansions": c(Counter::AutoExpansions as usize),
		"live_containers": c(Counter::LiveContainers as usize),
		"engine_archives": c(Counter::EngineArchives as usize),
		"retrieves": c(Counter::Retrieves as usize),
		"retrieve_misses": c(Counter::RetrieveMisses as usize),
		"template_renders": c(Counter::TemplateRenders as usize),
		"tool_errors": c(Counter::ToolErrors as usize),
		"warnings": c(Counter::Warnings as usize),
		"timing": {
			"tool_transform_avg_us": Avg(&TimingSum[TimingCat::ToolTransform as usize], &TimingN[TimingCat::ToolTransform as usize]),
			"retrieve_avg_us": Avg(&TimingSum[TimingCat::Retrieve as usize], &TimingN[TimingCat::Retrieve as usize]),
			"pre_turn_avg_us": Avg(&TimingSum[TimingCat::PreTurn as usize], &TimingN[TimingCat::PreTurn as usize]),
			"render_avg_us": Avg(&TimingSum[TimingCat::Render as usize], &TimingN[TimingCat::Render as usize]),
		},
	})
}
