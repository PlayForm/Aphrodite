//! Decode one hook call, dispatch it, and encode the result.

use std::os::raw::c_char;

use crate::Hook::Call::{Decode, Dispatch, Encode, Error, Input::Struct::Struct};

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
/// A wedged session lock degrades to `Error::Fn::Fn` rather than blocking the
/// gateway, and a panic anywhere inside degrades to error JSON rather than
/// aborting the host process.
///
/// This is also the first place the subscriber is installed (`Debug::Init`):
/// the ABI export is a bare symbol, so the init belongs on the first real body
/// that runs, and every hook call after this one is already covered.
///
/// The three stages are named, not inlined: `Decode` produces the call's decoded
/// surface, `Dispatch` routes it under the session's bounded lock, `Encode`
/// serialises the answer. The ORDER is the contract - the session id is resolved
/// during decode, before any lock is taken, and the fail-open value is computed
/// only after `Dispatch` has already returned.
pub(crate) fn call(hook_name:*const c_char, args_json:*const c_char) -> *mut c_char {
	crate::Debug::Init::Fn::Fn();

	let Name = unsafe { crate::Guard::Fn::cstr_to_string(hook_name) };
	let Args = unsafe { crate::Guard::Fn::cstr_to_string(args_json) };

	crate::Guard::Fn::guarded(std::panic::AssertUnwindSafe(move || {
		let Input:Struct = match Decode::Fn::Fn(&Args) {
			Ok(Input) => Input,
			Err(e) => return crate::Guard::Fn::to_json_error(&format!("invalid args: {e:#}")),
		};

		// One span per hook call, with the hook name and the session id as
		// fields (§4.5): the session id is resolved during decode, BEFORE any
		// lock is taken, so entering the span cannot widen a lock scope.
		let Span = tracing::info_span!("hook_call", hook = %Name, session = %Input.Session);
		let _Entered = Span.enter();

		// `Dispatch` returns None when THIS session's lock is wedged by an
		// abandoned worker - fail open with the per-hook degrade value instead
		// of blocking the gateway.
		let Result = match Dispatch::Fn::Fn(&Name, &Input) {
			Some(Result) => Result,
			None => Error::Fn::Fn(&Name, &Input),
		};

		Encode::Fn::Fn(&Result)
	}))
}
