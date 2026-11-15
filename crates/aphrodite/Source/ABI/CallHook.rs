//! `AphroditeCallHook` - the retired stateless hook entry point.

use std::os::raw::c_char;

use super::Guard::ToJSONError;

/// Deprecated (report 01-T5): stateless, creates a throwaway AphroditeState
/// per call. Use `AphroditeInit` + `AphroditeDispatch` (stateful, handle-based)
/// instead. This stub keeps the ABI symbol alive for any legacy consumer that
/// hasn't migrated yet.
///
/// It always answers with that deprecation message. It is not a silent fallback
/// to the stateless path: a caller that ignores the error would otherwise get
/// hook results computed against an empty session and believe them.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeCallHook(_Hook:*const c_char, _Args:*const c_char) -> *mut c_char {
	ToJSONError("aphrodite_call_hook is stateless and deprecated; use aphrodite_init + aphrodite_dispatch")
}
