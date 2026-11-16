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

/// Expand to a `#[unsafe(no_mangle)] pub unsafe extern "C"` export taking a
/// session handle followed by N C-string arguments.
///
/// Two stages, because they happen on two sides of the panic boundary. The
/// `unsafe` decode happens in the export's own frame, where the caller's pointer
/// contract is in force; resolving each decoded argument to a `Cow<str>` happens
/// inside the closure, where a refusal can become an error payload. Splitting them
/// is what lets the ingress cap be checked before the closure runs at all.
///
/// The generated signature is `unsafe` for the same reason the hand-written
/// exports are: the pointer obligations are the caller's, so they belong in the
/// signature rather than only in the documentation. A body therefore returns
/// `anyhow::Result<ABI::Boundary::Answer::Enum>` and never a pointer: no call site
/// can disagree about the response shape or forget the panic boundary.
///
/// Doc attributes written before the identifier are forwarded to the generated
/// function, which is the only way an expansion can carry `///` docs - including
/// the `# Safety` section each of its users needs.
macro_rules! stateful {
	($(#[$meta:meta])* $name:ident, |$s:ident, $($arg:ident : $ty:ty),*| $body:expr) => {
		$(#[$meta])*
		#[unsafe(no_mangle)]
		pub unsafe extern "C" fn $name(Handle:*const c_char, $($arg:*const c_char),*) -> *mut c_char {
			$(let $arg = unsafe { $crate::ABI::Guard::Cstr::Fn::Fn($arg) };)*
			$crate::ABI::Boundary::State::Fn::Fn(Handle, move |$s| {
				$(let $arg = $crate::ABI::Guard::Cstr::Optional::Fn::Fn($arg)?;)*
				$body
			})
		}
	};
}
