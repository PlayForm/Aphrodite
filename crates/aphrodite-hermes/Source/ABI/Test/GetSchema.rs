//! `aphrodite_hermes_get_schema`: a known tool returns its schema.

use std::ffi::{CStr, CString};

use crate::{aphrodite_hermes_free_string, aphrodite_hermes_get_schema};

#[test]
fn test_get_schema_known_tool() {
	let name = CString::new("aphrodite_compress").unwrap();
	let result_ptr = aphrodite_hermes_get_schema(name.as_ptr());
	let result = unsafe { CStr::from_ptr(result_ptr) }.to_string_lossy().into_owned();
	let v:serde_json::Value = serde_json::from_str(&result).unwrap();
	assert_eq!(v["name"], "aphrodite_compress");
	aphrodite_hermes_free_string(result_ptr);
}
