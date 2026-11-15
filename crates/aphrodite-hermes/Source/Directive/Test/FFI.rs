//! FFI round-trip: the exported C ABI entry point drives resolution and
//! materialization end to end.
//!
//! Everything below the export is covered by the area tests; what this pins is
//! the wire between them - that the pointer handed back is a readable JSON
//! string, that it reports the same file count the internal path does, and that
//! freeing it does not disturb the allocator.

use std::{
	ffi::{CStr, CString},
	fs,
};

use super::{Expected, Temp};

#[test]
fn ffi_materialize_directives_roundtrip() {
	let home = Temp::Fn("ffi");
	let home_c = CString::new(home.to_str().unwrap()).unwrap();
	let ptr = crate::aphrodite_hermes_materialize_directives(home_c.as_ptr());
	assert!(!ptr.is_null());
	let json = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
	crate::aphrodite_hermes_free_string(ptr);
	let v:serde_json::Value = serde_json::from_str(&json).unwrap();
	assert_eq!(v["status"], "ok");
	assert_eq!(
		v["written"].as_array().unwrap().len(),
		Expected::Fn().len(),
		"all embedded files must be written: {v}"
	);
	assert!(
		home.join("directives").join("focus.md").is_file(),
		"focus.md must be materialized as <home>/directives/focus.md"
	);
	fs::remove_dir_all(&home).ok();
}
