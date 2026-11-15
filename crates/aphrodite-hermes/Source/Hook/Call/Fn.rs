//! Decode one hook call, dispatch it, and encode the result.

use std::os::raw::c_char;

use crate::{
	Guard::Fn::{cstr_to_string, guarded, to_json_error},
	Hook::Call::{Decode::Fn::decode, Dispatch::Fn::dispatch, Encode::Fn::encode, Error::Fn::fail_open_hook},
};

/// Call a Hermes hook by name with JSON args.
///
/// Operates on the per-session state (see `Session/`) and honors the exact
/// Hermes hook contract (verified against the Hermes source):
///   - `transform_tool_result` - tool output arrives under `result`; return a
///     marker string to replace it, or `null` to pass through.
///   - `transform_terminal_output` - output arrives under `output`; same
///     return.
///   - `pre_llm_call` - return `{"context": "..."}` to inject a catalog
///     summary.
///   - `on_session_start` / `post_llm_call` - lifecycle; return value ignored.
///
/// A wedged session lock degrades to [`fail_open_hook`] rather than blocking
/// the gateway, and a panic anywhere inside degrades to error JSON rather than
/// aborting the host process.
///
/// This is also the first place the subscriber is installed (`Debug::Init`):
/// the ABI export is a bare symbol, so the init belongs on the first real body
/// that runs, and every hook call after this one is already covered.
pub(crate) fn call(hook_name:*const c_char, args_json:*const c_char) -> *mut c_char {
	crate::Debug::Init::Fn::Fn();

	let name = unsafe { cstr_to_string(hook_name) };
	let args = unsafe { cstr_to_string(args_json) };

	guarded(std::panic::AssertUnwindSafe(move || {
		let input = match decode(&args) {
			Ok(input) => input,
			Err(e) => return to_json_error(&format!("invalid args: {e:#}")),
		};

		// One span per hook call, with the hook name and the session id as
		// fields (§4.5): the session id is resolved during decode, BEFORE any
		// lock is taken, so entering the span cannot widen a lock scope.
		let Span = tracing::info_span!("hook_call", hook = %name, session = %input.session_id);
		let _Entered = Span.enter();

		// `dispatch` returns None when THIS session's lock is wedged by an
		// abandoned worker - fail open with the per-arm degrade value instead
		// of blocking the gateway.
		let result = match dispatch(&name, &input) {
			Some(v) => v,
			None => fail_open_hook(&name, &input.session_id, input.tool_content(), input.term_content()),
		};

		encode(&result)
	}))
}
