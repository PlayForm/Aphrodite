//! `aphrodite_stats` - live session counters plus proxy health.

use crate::{Health::Proxy::Fn::proxy_health, Tool::Dispatch::Fn::Handler};

/// The `aphrodite_stats` handler.
///
/// One lock acquisition covers every session field; the proxy poll runs after the
/// guard is dropped, because a TCP connect while holding the state mutex would
/// stall every other hook for up to the connect timeout.
///
/// The telemetry fields (`chain_split_*`) are diagnostics only - nothing here is
/// rendered into the model's conversational view.
pub(crate) fn Fn() -> (&'static str, Handler) {
	("aphrodite_stats", |_Args| {
		let mut Stats = crate::Session::With::Fn::with_shared(|State| {
			serde_json::json!({
				"version": env!("CARGO_PKG_VERSION"),
				"engine": "aphrodite-hermes",
				"inline_entries": State.InlineStore.len(),
				"markers": State.RecentMarkers.len(),
				"referenced_files": State.ReferencedFiles.len(),
				"archived_turns": State.ConvIndex.len(),
				"turn": State.TurnCounter,
				"engine_enabled": State.ContextEngineEnabled,
				"threshold_pct": State.EngineThresholdPct,
				"tool_threshold": State.ToolThreshold,
				"terminal_threshold": State.TerminalThreshold,
				"auto_reload": State.AutoReload,
				// Config self-diagnosis: present when aphrodite.toml was
				// found but failed to parse - defaults are in effect.
				"config_error": State.ConfigurationError,
				// Tier 1 teaching loop telemetry: the adaptive split
				// threshold and the consequence ledger. Diagnostics only -
				// never rendered into the LLM's conversational view.
				"chain_split_min_segments": State.ChainSplitMinimumSegments,
				"chain_split_events": State.SplitEvents.len(),
				"chain_split_produced": State.SplitEvents.iter().map(|Event| Event.Produced).sum::<usize>(),
				"chain_split_retrieved": State.SplitEvents.iter().map(|Event| Event.Retrieved).sum::<usize>(),
				// DEV-ONLY debug logger (temporary instrumentation): the
				// env-gated level + cumulative metrics snapshot.
				"debug_level": State.DebugLevel,
				"debug_metrics": aphrodite::Debug::Metrics::Fn::Fn(),
			})
		});

		Stats["proxies"] = proxy_health();

		Stats
	})
}
