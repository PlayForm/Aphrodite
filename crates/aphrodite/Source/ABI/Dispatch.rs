//! `AphroditeDispatch` - the universal entry point every host hook routes
//! through.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, Handle::Fn::Fn as HandleID, Run::Fn::Fn as Run},
	Guard::Cstr::Fn::Fn as Cstr,
	Handle::Lookup::WithState,
	Route::{Decode::Fn::Fn as Decode, Prepare::Fn::Fn as Prepare, Run::Fn::Fn as RunRoute},
};

/// Universal hook dispatcher. Python calls this for every hook handler.
/// Returns JSON-wrapped result or raw string if content-only.
///
/// The hook name is the routing key: the six hook names, the read-only
/// catalogue queries (`catalog`, `Stats`, `Diff`, `Files`, `search`,
/// `Classify`, `directive`) and `prefetch` all arrive here. An unrecognised name
/// answers `{"error":"unknown hook: ..."}` rather than an empty result, so a
/// host that misspells a hook learns it immediately.
///
/// Both `OnSessionStart` and the bare `session_start` are accepted, because
/// hosts in the wild register either; only the prefixed form appears in
/// `AphroditeHooks`.
///
/// This is the one export that composes the boundary itself rather than using
/// `Boundary::State`, because the span it emits carries the handle and the
/// payload size, and both are only known between the decode and the run. The
/// stages are `Route::Decode` (pure), `Route::Prepare` (the prefetch file reads,
/// before the lock) and `Route::Run` (under the lock), in that order, and that
/// order is the error precedence: a bad handle wins over bad args, bad args win
/// over oversize content, and an oversize payload never reaches the filesystem.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeDispatch(
	Handle:*const c_char,
	HookName:*const c_char,
	ArgsJSON:*const c_char,
) -> *mut c_char {
	let Hid = unsafe { HandleID(Handle) };
	let Name = unsafe { Cstr(HookName) }.unwrap_or_default();
	let ArgsStr = unsafe { Cstr(ArgsJSON) }.unwrap_or_default();

	Run(move || {
		let Hid = Hid.ok_or_else(|| anyhow::anyhow!("invalid handle"))?;

		let Args:serde_json::Value =
			serde_json::from_str(&ArgsStr).map_err(|Error| anyhow::anyhow!("invalid args: {}", Error))?;

		// Decode enforces the payload cap, so an oversize request is refused
		// before the span is opened and before any file is read.
		let Command = Decode(&Name, &Args)?;

		let Tool = Args.get("tool_name").and_then(|Value| Value.as_str()).unwrap_or("unknown");
		let Content = Args.get("content").and_then(|Value| Value.as_str()).unwrap_or("");

		// One span per host call. The hook name is the routing key and the handle
		// is the session, so a trace line answers "which session asked for what";
		// the content is measured, never logged, and `ok` is filled in at the end.
		let Span = tracing::info_span!(
			"abi.dispatch",
			hook = %Name,
			tool = %Tool,
			handle = Hid,
			content_bytes = Content.len(),
			ok = tracing::field::Empty,
		);
		let _Entered = Span.enter();

		// F9: the prefetch file reads happen here, before the session lock -
		// up to 10MB per file was previously read while holding the global
		// handle-map mutex, serializing every other session's every call behind
		// this one's (possibly slow/cold-mount) disk reads.
		let Prepared = Prepare(Command);

		let Outcome = WithState(Hid, |s| Ok(RunRoute(s, Prepared)));

		Span.record("ok", Outcome.is_ok());

		Outcome.map(Answer::Envelope)
	})
}
