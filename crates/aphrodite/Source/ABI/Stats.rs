//! `AphroditeStats` - the session's own counters, plus its configuration
//! self-diagnosis.

use std::os::raw::c_char;

use crate::ABI::Boundary::{Answer::Enum::Enum as Answer, State::Bare::Fn::Fn as Bare};

/// Answer with the session's counters and the configuration actually in
/// effect.
///
/// `ConfigurationError` is the field that matters when the numbers look wrong: it is
/// present exactly when an `aphrodite.toml` was found but failed to parse, so
/// the caller is reading defaults rather than what it configured. A host that
/// only logs the counters loses the one signal that explains them.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeStats(Handle:*const c_char) -> *mut c_char {
	Bare(Handle, |s| {
		Ok(Answer::Envelope(serde_json::json!({
			"version":env!("CARGO_PKG_VERSION"),"inline_entries":s.InlineStore.len(),
			"markers":s.RecentMarkers.len(),"turn":s.TurnCounter,
			"engine_enabled":s.ContextEngineEnabled,"threshold_pct":s.EngineThresholdPct,
			"tool_threshold":s.ToolThreshold,"terminal_threshold":s.TerminalThreshold,
			"auto_reload":s.AutoReload,
			// Config self-diagnosis: present when aphrodite.toml was
			// found but failed to parse - defaults are in effect.
			"config_error":s.ConfigurationError,
		})))
	})
}
