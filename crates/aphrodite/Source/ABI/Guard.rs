//! Hardened primitives every C ABI export calls before it touches session
//! state: the panic boundary, the two JSON response builders, the caller
//! string reader, the content validator, and the payload cap.
//!
//! Nothing in this file is exported itself. It exists so the panic rule has one
//! home: an export either routes through `Handle::Lookup::with_state` (which
//! guards internally) or wraps its body in `Guarded` - there is no third shape
//! that stays panic-safe across the FFI boundary.

use std::{
	ffi::{CStr, CString},
	os::raw::c_char,
};

/// Largest payload any single export accepts, in bytes (16 MB).
///
/// A C ABI caller hands over an already-materialised string, so an unbounded
/// length is an out-of-memory request rather than a slow path. The compression
/// and hook entry points (`AphroditeCompress`, `AphroditeTransform`,
/// `AphroditeTerminal`, `AphroditeDispatch`) and `AphroditeClassify` reject
/// anything larger with an error payload instead of attempting the copy; the
/// stateless text utilities (`FilterLines`, `Preview`, `stage2`,
/// `struct_extract`) do not check it, so the cap is a guard on the paths that
/// store, not a universal limit.
pub(crate) const MaximumContent:usize = 16 * 1024 * 1024; // 16MB cap

/// Build the `{"error": <msg>}` payload every failure path returns.
///
/// A null return is the only other failure shape, and it means `CString::new`
/// refused the message itself (an interior NUL in an error string): the caller
/// sees "no result" rather than a crash.
pub(crate) fn ToJSONError(Message:&str) -> *mut c_char {
	let JSON = serde_json::json!({"error": Message}).to_string();
	CString::new(JSON).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// Build the JSON payload for a successful call.
pub(crate) fn ToJSONOk(v:&serde_json::Value) -> *mut c_char {
	CString::new(v.to_string())
		.map(|c| c.into_raw())
		.unwrap_or(std::ptr::null_mut())
}

/// Run `f` under `catch_unwind`, converting a panic into an error-JSON
/// `*mut c_char` instead of unwinding across the `extern "C"` boundary
/// (which triggers the Rust runtime's forced process abort). `WithState`
/// already gives stateful fns this guarantee; this covers every extern fn
/// that doesn't go through `WithState` (classify, retrieve, filter_lines,
/// preview, stage2, struct_extract, init, catalog, stats, search, directive,
/// config_get) - previously several of these could abort the host process
/// on a panic (e.g. the byte-slicing panics in struct_extract.rs/marker.rs,
/// now fixed separately, but any future panic in these paths would have the
/// same effect).
///
/// Rule: every new `pub extern "C" fn` added to this crate must either route
/// through `WithState` (which guards internally) or wrap its body in
/// `guarded(AssertUnwindSafe(...))` directly - there is no third option that
/// stays panic-safe across the FFI boundary.
pub(crate) fn Guarded(f:impl FnOnce() -> *mut c_char + std::panic::UnwindSafe) -> *mut c_char {
	std::panic::catch_unwind(f).unwrap_or_else(|_| ToJSONError("internal error: panicked"))
}

/// Convert a caller-supplied C string pointer into an owned `String`.
///
/// A null pointer yields `None`, and each export decides for itself whether
/// that means "argument absent" (substituted with an empty default) or "request
/// invalid" (answered with an error payload).
///
/// # Safety
/// `Ptr`, if non-null, must point to a valid, NUL-terminated C string that
/// stays valid for the duration of this call (the standard `CStr::from_ptr`
/// contract). Every caller in this crate passes pointers received directly
/// from the C ABI call, which are expected to uphold that contract.
pub(crate) unsafe fn Cstr(Ptr:*const c_char) -> Option<String> {
	if Ptr.is_null() {
		return None;
	}
	// SAFETY: `Ptr` is non-null here, and per the safety contract of this
	// function it points to a valid, NUL-terminated C string that remains
	// valid for the duration of this call.
	Some(unsafe { CStr::from_ptr(Ptr) }.to_string_lossy().into_owned())
}

/// Reject a payload no export can serve: empty, over the 16 MB cap, or
/// carrying an interior NUL that would silently truncate the C string handed
/// back to the caller.
///
/// The three refusals are distinct messages on purpose - a host marshalling its
/// own buffers needs to tell "sent nothing" from "sent too much" from "sent a
/// NUL". The caller renders them through `format!("{e:#}")` into the JSON error
/// payload, so the message is the whole diagnosis the C side gets.
pub(crate) fn CheckContent(Content:&str) -> anyhow::Result<()> {
	if Content.is_empty() {
		anyhow::bail!("empty content");
	}
	if Content.len() > MaximumContent {
		anyhow::bail!("content exceeds 16MB limit");
	}
	if Content.contains('\0') {
		anyhow::bail!("content contains null bytes");
	}
	Ok(())
}
