//! The fail-open value: what a hook returns when it could not run.
//!
//! This is the second half of the gateway-wedge fix. When the bounded lock
//! reports a wedged session, the hook must still answer, and it must answer
//! with the value that changes NOTHING: a transform returns the original
//! content verbatim (Hermes honors a string return as the replacement, so
//! returning the identical content is a visible no-op) rather than erasing the
//! output, and every other hook returns `null`.
//!
//! The decoded call is the input, not four loose strings: the transform that
//! must be echoed back is a property of WHICH hook fired, and this leaf reads it
//! off the same value the arms read rather than trusting a caller to pass the
//! matching content.

use crate::Hook::Call::Input::Struct::Struct;

/// Fail-open degrade value for a hook whose session lock is wedged by an
/// abandoned worker (the bounded lock returned None). Never blocks and never
/// erases output: `pre_tool_call` -> null (no directive; the tool proceeds
/// normally), `transform_tool_result` / `transform_terminal_output` -> the
/// ORIGINAL content string (Hermes honors a string return as the
/// replacement; returning the identical content is a visible no-op),
/// `pre_llm_call` -> null (no context injection), lifecycle hooks -> null
/// (return ignored).
///
/// The span carries the hook name and the session id (§4.5): a wedged session is
/// the one degradation an operator has to be able to attribute from the log
/// alone, and the return values below are what keeps the gateway alive while
/// they go looking. `session` is a field only - it changes nothing about what is
/// returned.
pub(crate) fn Fn(Name:&str, Input:&Struct) -> serde_json::Value {
	let Span = tracing::info_span!("fail_open_hook", hook = Name, session = Input.Session.as_str());
	let _Entered = Span.enter();

	// A wedged session lock is a DEGRADATION, not a routine path: the hook
	// answered without doing its job. Warn (never error - the gateway is fine)
	// and name the hook; the content itself is never logged.
	tracing::warn!(
		target: crate::Debug::Init::Const::TARGET,
		hook = Name,
		"session lock wedged; degrading fail-open (nothing blocked, nothing erased)"
	);

	match Name {
		"transform_tool_result" => serde_json::Value::String(Input.Content().to_string()),
		"transform_terminal_output" => serde_json::Value::String(Input.Output().to_string()),
		_ => serde_json::Value::Null,
	}
}
