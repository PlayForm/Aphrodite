//! `AphroditeCallHook` - the retired stateless hook entry point.

use std::os::raw::c_char;

use crate::ABI::Boundary::Run::Fn::Fn as Run;

/// Deprecated (report 01-T5): stateless, creates a throwaway AphroditeState
/// per call. Use `AphroditeInit` + `AphroditeDispatch` (stateful, handle-based)
/// instead. This stub keeps the ABI symbol alive for any legacy consumer that
/// hasn't migrated yet.
///
/// It always answers with that deprecation message. It is not a silent fallback
/// to the stateless path: a caller that ignores the error would otherwise get
/// hook results computed against an empty session and believe them.
///
/// The refusal is an `anyhow::Error` rather than a pre-rendered envelope, so the
/// message has one spelling and the boundary owns its rendering.
/// # Safety
/// Both pointers are never dereferenced - the export answers its deprecation
/// message without reading either - so any value is acceptable, including null.
/// The returned pointer is owned by the caller and must be released with
/// `AphroditeFreeString`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn AphroditeCallHook(_Hook:*const c_char, _Args:*const c_char) -> *mut c_char {
	Run(|| {
		Err(anyhow::anyhow!(
			"aphrodite_call_hook is stateless and deprecated; use aphrodite_init + aphrodite_dispatch"
		))
	})
}
