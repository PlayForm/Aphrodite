//! `AphroditeReload` - re-read one configuration key from disk.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, State::Fn::Fn as State},
	Configuration::Reload::{Apply::Fn::Fn as Apply, Assess::Fn::Fn as Assess},
	Guard::Cstr::Fn::Fn as Cstr,
};

/// Re-read `[compression] context_engine` from `path` and apply it.
///
/// Degrades rather than fails: a missing or unreadable file is the normal search
/// miss and changes nothing, and a file that exists but does not parse keeps the
/// previous value, warns on a real log surface, records the failure on the
/// session (so `AphroditeStats` self-diagnoses) and echoes it in this
/// response's `ConfigurationError`. The status stays `"ok"` in every case, because a
/// host must not stop running over a config file it can simply ignore.
///
/// The file is read and parsed BEFORE the session lock is taken (F9): this fn
/// used to do the read inside `WithState`'s closure, holding the *global*
/// handle-map mutex across disk I/O - on a slow/cold mount, one handle's
/// reload blocked every other handle's every call, and it was a trap for a
/// future closure that called back into another `aphrodite_*` fn (non-reentrant
/// Mutex -> instant deadlock).
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeReload(Handle:*const c_char, path:*const c_char) -> *mut c_char {
	let Path = unsafe { Cstr(path) }.unwrap_or_default();

	let Assessment = Assess(&Path);

	State(Handle, move |s| {
		let Diagnostic = Apply(s, &Assessment);

		Ok(Answer::Envelope(serde_json::json!({"status":"ok", "config_error":Diagnostic})))
	})
}
