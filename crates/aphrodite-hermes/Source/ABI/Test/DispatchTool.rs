//! `aphrodite_hermes_dispatch_tool`: unknown tools and malformed args must both
//! come back as error JSON, never as an abort.

use std::ffi::{CStr, CString};

use crate::{aphrodite_hermes_dispatch_tool, aphrodite_hermes_free_string};

#[test]
fn test_dispatch_unknown_tool() {
	let name = CString::new("nonexistent").unwrap();
	let args = CString::new("{}").unwrap();
	let result_ptr = aphrodite_hermes_dispatch_tool(name.as_ptr(), args.as_ptr());
	let result = unsafe { CStr::from_ptr(result_ptr) }.to_string_lossy().into_owned();
	assert!(result.contains("error"));
	aphrodite_hermes_free_string(result_ptr);
}

#[test]
fn test_call_hook_panic_path_returns_error_not_abort() {
	// dispatch_tool with a tool name that panics in the registry lookup
	// path would abort pre-T2; now it must come back as error JSON.
	let name = CString::new("aphrodite_compress").unwrap();
	// Malformed args (missing required fields) exercise the same guarded
	// path without relying on a specific internal panic site.
	let args = CString::new("not json").unwrap();
	let ptr = aphrodite_hermes_dispatch_tool(name.as_ptr(), args.as_ptr());
	assert!(!ptr.is_null());
	aphrodite_hermes_free_string(ptr);
}
