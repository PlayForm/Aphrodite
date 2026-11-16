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
				"inline_entries": State.Store.Map.len(),
				"markers": State.Marker.Recent.len(),
				"referenced_files": State.File.Recent.len(),
				"archived_turns": State.Turn.Index.len(),
				"turn": State.Turn.Counter,
				"engine_enabled": State.Configuration.ContextEngineEnabled,
				"threshold_pct": State.Configuration.EngineThresholdPct,
				"tool_threshold": State.Configuration.ToolThreshold,
				"terminal_threshold": State.Configuration.TerminalThreshold,
				"auto_reload": State.Configuration.AutoReload,
				// Config self-diagnosis: present when aphrodite.toml was
				// found but failed to parse - defaults are in effect.
				"config_error": State.Configuration.Error,
				// Tier 1 teaching loop telemetry: the adaptive split
				// threshold and the consequence ledger. Diagnostics only -
				// never rendered into the LLM's conversational view.
				"chain_split_min_segments": State.ChainSplit.Minimum,
				"chain_split_events": State.ChainSplit.Ledger.len(),
				"chain_split_produced": State.ChainSplit.Ledger.iter().map(|Event| Event.Produced).sum::<usize>(),
				"chain_split_retrieved": State.ChainSplit.Ledger.iter().map(|Event| Event.Retrieved).sum::<usize>(),
				// DEV-ONLY debug logger (temporary instrumentation): the
				// env-gated level + cumulative metrics snapshot.
				"debug_level": State.Configuration.DebugLevel,
				"debug_metrics": aphrodite::Debug::Metrics::Fn::Fn(),
			})
		});

		Stats["proxies"] = proxy_health();

		Stats
	})
}
