//! `aphrodite_hermes_materialize_directives` - provision the builtin directives.

use std::os::raw::c_char;

use crate::{
	Directive,
	Guard::{
		Fn::{cstr_to_string, guarded},
		ToJSON::Fn::Fn as to_json,
	},
};

/// Materialize the embedded behavioral directives into
/// `<runtime-home>/directives/` (idempotent; never overwrites user data).
///
/// The shipped directive set is embedded in the core `aphrodite` crate
/// (`builtin_directives/*.md` via `include_str!`, exposed through
/// `aphrodite::Directive::Builtin::Fn()`); this fn provisions it into
/// the user-data home so the plugin directory never holds runtime state.
///
/// `home_dir` is the runtime home (the user-data folder, e.g.
/// `~/.hermes/aphrodite`). If null/empty it is resolved from
/// `$APHRODITE_DIRECTIVES_DIR` (exact dir), `$APHRODITE_HOME` (home), then
/// `$HOME/.hermes/aphrodite`. Returns JSON
/// `{"status":"ok","dir":...,"written":[...],"skipped":[...],"warnings":[...]}`
/// - always `status:"ok"` (failures degrade to warnings). Caller must free
///   with `aphrodite_hermes_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_materialize_directives(home_dir:*const c_char) -> *mut c_char {
	let home = unsafe { cstr_to_string(home_dir) };
	guarded(std::panic::AssertUnwindSafe(move || {
		let result = Directive::Into::Fn::Fn(&home);
		to_json(&result)
	}))
}
