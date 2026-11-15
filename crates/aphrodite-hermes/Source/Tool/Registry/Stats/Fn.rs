//! `aphrodite_stats` - live session counters plus proxy health.

use crate::{Tool::Dispatch::ToolHandler, proxy_health};

/// The `aphrodite_stats` handler.
///
/// One lock acquisition covers every session field; the proxy poll runs after the
/// guard is dropped, because a TCP connect while holding the state mutex would
/// stall every other hook for up to the connect timeout.
///
/// The telemetry fields (`chain_split_*`) are diagnostics only - nothing here is
/// rendered into the model's conversational view.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("aphrodite_stats", |_args| {
		let mut stats = crate::with_shared(|state| {
			serde_json::json!({
				"version": env!("CARGO_PKG_VERSION"),
				"engine": "aphrodite-hermes",
				"inline_entries": state.inline_store.len(),
				"markers": state.recent_markers.len(),
				"referenced_files": state.referenced_files.len(),
				"archived_turns": state.conv_index.len(),
				"turn": state.turn_counter,
				"engine_enabled": state.context_engine_enabled,
				"threshold_pct": state.engine_threshold_pct,
				"tool_threshold": state.tool_threshold,
				"terminal_threshold": state.terminal_threshold,
				"auto_reload": state.auto_reload,
				// Config self-diagnosis: present when aphrodite.toml was
				// found but failed to parse - defaults are in effect.
				"config_error": state.config_error,
				// Tier 1 teaching loop telemetry: the adaptive split
				// threshold and the consequence ledger. Diagnostics only -
				// never rendered into the LLM's conversational view.
				"chain_split_min_segments": state.chain_split_min_segments,
				"chain_split_events": state.split_events.len(),
				"chain_split_produced": state.split_events.iter().map(|e| e.produced).sum::<usize>(),
				"chain_split_retrieved": state.split_events.iter().map(|e| e.retrieved).sum::<usize>(),
				// DEV-ONLY debug logger (temporary instrumentation): the
				// env-gated level + cumulative metrics snapshot.
				"debug_level": state.debug_level,
				"debug_metrics": aphrodite::Debug::Metrics::Fn::Fn(),
			})
		});
		stats["proxies"] = proxy_health();
		stats
	})
}
