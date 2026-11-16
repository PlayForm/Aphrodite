//! `[compression]` engine thresholds, plus the `[defaults]` upstream values.

use super::super::super::Struct::Struct;
use crate::State::Struct::AphroditeState;

/// Write the context-engine thresholds and the upstream model/URL defaults.
///
/// Each knob is resolved independently, so a file that sets only
/// `EngineMinimumMsgs` still gets every other default.
pub(crate) fn Apply(Configuration:&Struct, State:&mut AphroditeState) {
	State.Configuration.ContextEngineEnabled =
		Configuration.Bool("APHRODITE_CONTEXT_ENGINE", "compression", "context_engine", true);
	State.Configuration.EngineThresholdPct =
		Configuration.Number("APHRODITE_ENGINE_THRESHOLD_PCT", "compression", "engine_threshold_pct", 45);
	State.Configuration.EngineMinimumMsgs =
		Configuration.Size("APHRODITE_ENGINE_MIN_MSGS", "compression", "engine_min_msgs", 8);
	State.Configuration.EngineProtectFirst =
		Configuration.Size("APHRODITE_ENGINE_PROTECT_FIRST", "compression", "engine_protect_first", 2);
	State.Configuration.EngineProtectLast =
		Configuration.Size("APHRODITE_ENGINE_PROTECT_LAST", "compression", "engine_protect_last", 5);
	// F3: was keyed "tool_threshold" and env var APHRODITE_TOOL_THRESHOLD -
	// neither exists in any shipped TOML/doc (they use
	// tool_threshold_token/tool_threshold_cache); wiring this up as-is
	// would always fall through to the default. The Hermes dylib path
	// is the tool-injecting, aggressive-compression environment (no
	// cache-vs-token split like the proxy's dual listeners), so it maps
	// to `ToolThresholdToken`.
	State.Configuration.ToolThreshold =
		Configuration.Size("APHRODITE_TOOL_THRESHOLD_TOKEN", "compression", "tool_threshold_token", 4096);
	State.Configuration.TerminalThreshold =
		Configuration.Size("APHRODITE_TERMINAL_THRESHOLD", "compression", "terminal_threshold", 1024);
	State.Configuration.Model = Configuration.Text("APHRODITE_MODEL", "defaults", "model", "gpt-4o");
	State.Configuration.URL = Configuration.Text("APHRODITE_API_URL", "defaults", "api_url", "");
}
