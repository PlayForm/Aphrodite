//! `aphrodite_stats` - live session counters plus proxy health.

use crate::{Health::Proxy::Fn::proxy_health, Tool::Dispatch::Fn::ToolHandler};

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
		let mut stats = crate::Session::With::Fn::with_shared(|state| {
			serde_json::json!({
				"version": env!("CARGO_PKG_VERSION"),
				"engine": "aphrodite-hermes",
				"inline_entries": state.InlineStore.len(),
				"markers": state.RecentMarkers.len(),
				"referenced_files": state.ReferencedFiles.len(),
				"archived_turns": state.ConvIndex.len(),
				"turn": state.TurnCounter,
				"engine_enabled": state.ContextEngineEnabled,
				"threshold_pct": state.EngineThresholdPct,
				"tool_threshold": state.ToolThreshold,
				"terminal_threshold": state.TerminalThreshold,
				"auto_reload": state.AutoReload,
				// Config self-diagnosis: present when aphrodite.toml was
				// found but failed to parse - defaults are in effect.
				"config_error": state.ConfigurationError,
				// Tier 1 teaching loop telemetry: the adaptive split
				// threshold and the consequence ledger. Diagnostics only -
				// never rendered into the LLM's conversational view.
				"chain_split_min_segments": state.ChainSplitMinimumSegments,
				"chain_split_events": state.SplitEvents.len(),
				"chain_split_produced": state.SplitEvents.iter().map(|e| e.Produced).sum::<usize>(),
				"chain_split_retrieved": state.SplitEvents.iter().map(|e| e.Retrieved).sum::<usize>(),
				// DEV-ONLY debug logger (temporary instrumentation): the
				// env-gated level + cumulative metrics snapshot.
				"debug_level": state.DebugLevel,
				"debug_metrics": aphrodite::Debug::Metrics::Fn::Fn(),
			})
		});
		stats["proxies"] = proxy_health();
		stats
	})
}
