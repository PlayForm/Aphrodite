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
/// The expansion reads every argument as a C string and hands the handle plus a
/// closure to `ABI::Boundary::State`, which decodes the handle, runs the closure
/// under the session lock and owns the envelope, the panic and the ownership
/// transfer. A body therefore returns
/// `anyhow::Result<ABI::Boundary::Answer::Enum>` and never a pointer: no call
/// site can disagree about the response shape or forget the panic boundary.
///
/// Doc attributes written before the identifier are forwarded to the generated
/// function, which is the only way an expansion can carry `///` docs of its
/// own.
macro_rules! stateful {
	($(#[$meta:meta])* $name:ident, |$s:ident, $($arg:ident : $ty:ty),*| $body:expr) => {
		$(#[$meta])*
		#[unsafe(no_mangle)]
		pub extern "C" fn $name(Handle:*const c_char, $($arg:*const c_char),*) -> *mut c_char {
			$(let $arg = unsafe { $crate::ABI::Guard::Cstr::Fn::Fn($arg) }.unwrap_or_default();)*
			$crate::ABI::Boundary::State::Fn::Fn(Handle, move |$s| $body)
		}
	};
}
