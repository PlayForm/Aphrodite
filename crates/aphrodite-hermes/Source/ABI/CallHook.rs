//! `aphrodite_hermes_call_hook` - the hook dispatch entry point.

use std::os::raw::c_char;

use crate::Hook::Call::Fn::call;

/// Call a Hermes hook by name with JSON args.
///
/// The body lives in `Hook::Call` (decode -> dispatch -> encode); this export is
/// the symbol boundary and nothing else, so the hook contract has exactly one
/// implementation. See `Hook::Call::Fn::call` for the per-hook return shapes.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_call_hook(hook_name:*const c_char, args_json:*const c_char) -> *mut c_char {
	call(hook_name, args_json)
}
