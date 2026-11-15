//! The terminal-output gate sequence.

use super::super::{ChainSplit, Emit, Record};
use crate::{
	Hook::{AutoExpand::Applies, Live::Container, ToolResult::Classify},
	State::Struct::AphroditeState,
};

/// The terminal-output pipeline: telemetry, the auto-expand gate, chain
/// splitting, the threshold gate, the live container, then store + marker.
///
/// Gate ORDER is the contract. Telemetry runs before every early return (a
/// failing command with tiny output is exactly what the error-loop breaker
/// needs); `DoAutoExpand` is computed once before the chain-split and
/// threshold gates so no marker-emitting path can run under it; chain-split
/// runs before the threshold gate because below-threshold marked output would
/// otherwise pass through with segment markers still in it, polluting what the
/// model sees. Reordering any of these changes what the model reads.
pub(crate) fn Fn(
	State:&mut AphroditeState,
	Content:&str,
	Classify:Option<(&str, &str)>,
	command:Option<&str>,
	Returncode:Option<i64>,
) -> serde_json::Value {
	// Telemetry (P2/T6): record before threshold/empty gating - a failing
	// command with tiny output is exactly what the error-loop breaker needs.
	match !Content.is_empty() {
		true => {
			Record::Fn::Fn(State, Content, command, Returncode);
		},
		false => {},
	}
	match Content.is_empty() {
		true => {
			return serde_json::json!({"status": "ok", "compressed": false, "reason": "empty"});
		},
		false => {},
	}

	// ── Auto-expand gate (pre-Rust port) ──
	// Computed once, BEFORE the chain-split / threshold gates: when enabled
	// and the content fits the limit, no marker-emitting path may run at
	// all - the content goes straight to the store+expand single path so
	// no `<<<CCR:...>>>` (and no chain-split segment marker) reaches the
	// model. Over the limit or disabled, every gate below behaves exactly
	// as before.
	let DoAutoExpand = Applies::Fn::Fn(State, Content);

	// ── Fine-grained chain splitting ──
	// Runs BEFORE the threshold gate: marked output must never leak raw
	// (below-threshold chained output would otherwise pass through with the
	// segment markers still in it, polluting what the LLM sees). Splitting
	// marked content is unconditional when the feature is enabled - except
	// under auto-expand (within the limit), where the whole content is
	// returned expanded instead.
	match ChainSplit::Fn::Fn(State, Content, DoAutoExpand) {
		Some(Payload) => {
			return Payload;
		},
		None => {},
	}

	// Threshold gate for UNMARKED output only: below-threshold content
	// passes through untouched (no markers to leak, nothing worth
	// splitting). Auto-expand bypasses the gate (store+expand path).
	match !DoAutoExpand && State.TerminalThreshold > 0 && Content.len() < State.TerminalThreshold {
		true => {
			return serde_json::json!({"status": "ok", "compressed": false, "reason": "below_threshold"});
		},
		false => {},
	}

	// ── Live container (pre-Rust port of _hooks/live.py) ──
	// Same contract as the tool path: enabled AND above the compress
	// threshold → chunked container store + INSTANT marker with a progress
	// annotation. Chain-split (above) already handled marked output, and
	// auto-expand wins when it applies.
	match State.LiveContainer && !DoAutoExpand {
		true => {
			let (TypeStr, _ClassifyContent) = Classify::Fn::Fn(Content, Classify, true);
			return Container::Fn::Fn(State, Content, &TypeStr);
		},
		false => {},
	}

	Emit::Fn::Fn(State, Content, Classify, DoAutoExpand)
}
