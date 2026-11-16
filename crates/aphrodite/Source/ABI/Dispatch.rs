//! `AphroditeDispatch` - the universal entry point every host hook routes
//! through.

use std::os::raw::c_char;

use crate::ABI::{
	Boundary::{Answer::Enum::Enum as Answer, Handle::Fn::Fn as HandleID, Run::Fn::Fn as Run},
	Guard::Cstr::{Fn::Fn as Cstr, Optional::Fn::Fn as Optional},
	Handle::Lookup::Guarded,
	Route::{Decode::Fn::Fn as Decode, Prepare::Fn::Fn as Prepare, Run::Fn::Fn as Execute},
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
/// # Safety
/// `Handle`, `Hook` and `Args` must each be null or point to a valid,
/// NUL-terminated C string that stays readable and unmoved for the duration of
/// this call. The returned pointer is owned by the caller and must be released
/// with `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeDispatch(
	Handle:*const c_char,
	Hook:*const c_char,
	Args:*const c_char,
) -> *mut c_char {
	let Identity = unsafe { HandleID(Handle) };
	let Name = unsafe { Cstr(Hook) };
	let Text = unsafe { Cstr(Args) };

	Run(move || {
		let Identity = Identity.ok_or_else(|| anyhow::anyhow!("invalid handle"))?;
		let Name = Optional(Name)?;
		let Text = Optional(Text)?;

		let Payload:serde_json::Value =
			serde_json::from_str(&Text).map_err(|Error| anyhow::anyhow!("invalid args: {}", Error))?;

		// Decode enforces the payload cap, so an oversize request is refused
		// before the span is opened and before any file is read.
		let Command = Decode(&Name, &Payload)?;

		let Tool = Payload.get("tool_name").and_then(|Value| Value.as_str()).unwrap_or("unknown");
		let Content = Payload.get("content").and_then(|Value| Value.as_str()).unwrap_or("");

		// One span per host call. The hook name is the routing key and the handle
		// is the session, so a trace line answers "which session asked for what";
		// the content is measured, never logged, and `ok` is filled in at the end.
		let Span = tracing::info_span!(
			"abi.dispatch",
			hook = %Name,
			tool = %Tool,
			handle = Identity,
			content_bytes = Content.len(),
			ok = tracing::field::Empty,
		);
		let _Entered = Span.enter();

		// F9: the prefetch file reads happen here, before the session lock -
		// up to 10MB per file was previously read while holding the global
		// handle-map mutex, serializing every other session's every call behind
		// this one's (possibly slow/cold-mount) disk reads.
		let Prepared = Prepare(Command);

		let Outcome = Guarded(Identity, |s| Ok(Execute(s, Prepared)));

		Span.record("ok", Outcome.is_ok());

		Outcome.map(Answer::JSON)
	})
}
