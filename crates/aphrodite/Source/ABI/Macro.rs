//! The `stateful!` macro: the one shape every handle-taking export shares.
//!
//! `macro_rules!` is textually scoped, so this module is `#[macro_use]`-
//! declared in `ABI/mod.rs` ahead of the leaves that expand it (`Compress`,
//! `Transform`, `Terminal`). Moving the declaration below them in that file
//! makes every call site fail with "cannot find macro `stateful` in this
//! scope" - the expansion is positional, not crate-wide.
//!
//! The generated function keeps the caller-supplied identifier verbatim: the
//! symbol name is the ABI contract that cbindgen, the plugin bindings and
//! `Maintain/check_ffi_contract.py` assert, so the macro may never rename it.

/// Expand to a `#[unsafe(no_mangle)] pub extern "C"` export taking a session
/// handle followed by N C-string arguments.
///
/// The expansion parses the handle, reads every argument as a C string, and
/// runs the body under `Handle::Lookup::with_state`, which is where the
/// torn-state contract and the panic boundary come from. A body therefore
/// returns a `serde_json::Value` and never a pointer: the macro owns both the
/// success and the error envelope, so no call site can disagree about the
/// response shape.
///
/// Doc attributes written before the identifier are forwarded to the generated
/// function, which is the only way an expansion can carry `///` docs of its
/// own.
macro_rules! stateful {
	($(#[$meta:meta])* $name:ident, |$s:ident, $($arg:ident : $ty:ty),*| $body:expr) => {
		$(#[$meta])*
		#[unsafe(no_mangle)]
		pub extern "C" fn $name(Handle:*const c_char, $($arg:*const c_char),*) -> *mut c_char {
			let Hid = match unsafe { $crate::ABI::Guard::Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
				Ok(ID) => ID,
				Err(_) => return $crate::ABI::Guard::ToJSONError("invalid handle"),
			};
			$(let $arg = unsafe { $crate::ABI::Guard::Cstr($arg) }.unwrap_or_default();)*
			match $crate::ABI::Handle::Lookup::WithState(Hid, |$s| $body) {
				Ok(v) => $crate::ABI::Guard::ToJSONOk(&v),
				Err(e) => $crate::ABI::Guard::ToJSONError(&format!("{e:#}")),
			}
		}
	};
}
