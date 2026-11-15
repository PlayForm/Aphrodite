//! `aphrodite_hermes_get_hooks` - the hook names this crate registers.

use std::os::raw::c_char;

use crate::Guard::Fn::{guarded, to_c_string};

/// Return hook names as a JSON array.
///
/// Hermes invokes the session hook as `on_session_start` (the `on_` prefix is
/// required by its VALID_HOOKS table); registering `session_start` silently
/// no-ops. The other four names match Hermes verbatim.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_get_hooks() -> *mut c_char {
	guarded(|| {
		to_c_string(
			&serde_json::json!([
				"on_session_start",
				"pre_tool_call",
				"transform_tool_result",
				"transform_terminal_output",
				"pre_llm_call",
				"post_llm_call"
			])
			.to_string(),
		)
	})
}
