//! `AphroditeReload` - re-read one configuration key from disk.

use std::os::raw::c_char;

use crate::ABI::{
	Guard::{Cstr, ToJSONError, ToJSONOk},
	Handle::Lookup::WithState,
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
/// Read the config file BEFORE taking the HANDLES lock (F9): this fn used
/// to do the read inside `WithState`'s closure, holding the *global*
/// handle-map mutex across disk I/O - on a slow/cold mount, one handle's
/// reload blocks every other handle's every call, and it's a trap for a
/// future `f` that calls back into another `aphrodite_*` fn (non-reentrant
/// Mutex -> instant deadlock).
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeReload(Handle:*const c_char, path:*const c_char) -> *mut c_char {
	let Hid = match unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Ok(ID) => ID,
		Err(_) => return ToJSONError("invalid handle"),
	};
	let p = unsafe { Cstr(path) }.unwrap_or_default();
	let mut ParseError:Option<String> = None;
	let ContextEngineEnabled = match !p.is_empty() {
		true => {
			match std::fs::read_to_string(p.as_str()) {
				Ok(Content) => {
					match Content.parse::<toml::Table>() {
						Ok(Tbl) => {
							Tbl.get("compression")
								.and_then(|v| v.as_table())
								.cloned()
								.and_then(|c| c.get("context_engine").and_then(|v| v.as_bool()))
						},
						Err(Error) => {
							// Issue #38 class: a found-but-broken TOML on the reload
							// path used to be discarded silently (`.parse().ok()`),
							// indistinguishable from "config not set". Warn on a
							// real log surface (stderr fallback when no tracing
							// subscriber exists) and record the failure on the
							// state + response so `AphroditeStats` can
							// self-diagnose.
							crate::ConfigurationLoader::Warn::Fn::WarnParseFailure(
								std::path::Path::new(p.as_str()),
								&Error,
								"keeping previous value",
							);
							ParseError = Some(format!("{p}: {Error}"));
							None
						},
					}
				},
				// Unreadable/missing is the normal search miss - keep silent.
				Err(_) => None,
			}
		},
		false => None,
	};
	match WithState(Hid, |s| {
		match ContextEngineEnabled {
			Some(v) => s.ContextEngineEnabled = v,
			None => (),
		}
		match &ParseError {
			// Surface "config: defaults/previous in effect (parse failed)"
			// on the same self-diagnosis field the loader uses.
			Some(e) => s.ConfigurationError = Some(e.clone()),
			None => (),
		}
		serde_json::json!({"status":"ok", "config_error":ParseError})
	}) {
		Ok(v) => ToJSONOk(&v),
		Err(e) => ToJSONError(&format!("{e:#}")),
	}
}
