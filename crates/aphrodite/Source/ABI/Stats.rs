//! `AphroditeStats` - the session's own counters, plus its configuration
//! self-diagnosis.

use std::os::raw::c_char;

use crate::ABI::Boundary::{Answer::Enum::Enum as Answer, State::Bare::Fn::Fn as Bare};

/// Answer with the session's counters and the configuration actually in
/// effect.
///
/// `Configuration::Error` is the field that matters when the numbers look wrong: it is
/// present exactly when an `aphrodite.toml` was found but failed to parse, so
/// the caller is reading defaults rather than what it configured. A host that
/// only logs the counters loses the one signal that explains them.
/// # Safety
/// `Handle` must be null or point to a valid, NUL-terminated C string that stays
/// readable and unmoved for the duration of this call. The returned pointer is
/// owned by the caller and must be released with `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeStats(Handle:*const c_char) -> *mut c_char {
	Bare(Handle, |s| {
		Ok(Answer::JSON(serde_json::json!({
			"version":env!("CARGO_PKG_VERSION"),"inline_entries":s.Store.Map.len(),
			"markers":s.Marker.Recent.len(),"turn":s.Turn.Counter,
			"engine_enabled":s.Configuration.ContextEngineEnabled,"threshold_pct":s.Configuration.EngineThresholdPct,
			"tool_threshold":s.Configuration.ToolThreshold,"terminal_threshold":s.Configuration.TerminalThreshold,
			"auto_reload":s.Configuration.AutoReload,
			// Config self-diagnosis: present when aphrodite.toml was
			// found but failed to parse - defaults are in effect.
			"config_error":s.Configuration.Error,
		})))
	})
}
