//! `aphrodite_hermes_free_string`: the allocate/free pairing must be symmetric
//! and allocator-safe.

use std::ffi::CStr;

use crate::{
	Guard::Fn::{to_c_string, to_json_error},
	aphrodite_hermes_free_string,
};

// ── F4 hardening: the FFI allocate/free pairing must be symmetric and
// allocator-safe. Every string returned across the C ABI is allocated
// with `CString::new(...).into_raw()` (`to_c_string`) and reclaimed by
// `CString::from_raw` inside `aphrodite_hermes_free_string` - never via a
// manual `libc::free`/`free()` or an allocator-specific dealloc. Because
// both halves route through the Rust global allocator (the crate installs
// no `#[global_allocator]`, so that is the default system allocator shared
// process-wide), the pairing holds regardless of which dylib image
// allocated and which freed it. This test round-trips fresh allocations
// (allocate → read back → free) repeatedly; an asymmetric pairing (wrong
// deallocator, double free, invalid free) would abort the harness, not
// silently pass. ──
#[test]
fn test_free_string_round_trip_symmetric() {
	// Varying lengths incl. empty string: allocate, read, free, repeat.
	for i in 0..256 {
		let payload = format!("round-trip-{i}-{}", "x".repeat(i % 64));
		let ptr = to_c_string(&payload);
		assert!(!ptr.is_null(), "to_c_string returned null for payload #{i}");
		let read = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		assert_eq!(read, payload, "read-back mismatch for payload #{i}");
		aphrodite_hermes_free_string(ptr);
	}
	// Empty string must round-trip too (CString is NUL-terminated).
	let ptr = to_c_string("");
	assert!(!ptr.is_null());
	assert_eq!(unsafe { CStr::from_ptr(ptr) }.to_bytes(), b"");
	aphrodite_hermes_free_string(ptr);
	// Error-JSON path (`to_json_error` → `to_c_string`) pairs identically.
	for i in 0..64 {
		let ptr = to_json_error(&format!("boom-{i}"));
		assert!(!ptr.is_null());
		let read = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		assert!(read.contains("boom-"), "error payload #{i} corrupted");
		aphrodite_hermes_free_string(ptr);
	}
	// Interior NUL: CString::new fails → null; free_string(null) is a no-op.
	let ptr = to_c_string("a\0b");
	assert!(ptr.is_null());
	aphrodite_hermes_free_string(ptr);
	aphrodite_hermes_free_string(std::ptr::null_mut());
}
