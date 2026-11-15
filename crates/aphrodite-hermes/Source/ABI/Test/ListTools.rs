//! `aphrodite_hermes_list_tools`: the registry must expose every tool.

use std::ffi::CStr;

use crate::{aphrodite_hermes_free_string, aphrodite_hermes_list_tools};

#[test]
fn test_list_tools_returns_array() {
	let json_ptr = aphrodite_hermes_list_tools();
	let json = unsafe { CStr::from_ptr(json_ptr) }.to_string_lossy().into_owned();
	let v:serde_json::Value = serde_json::from_str(&json).unwrap();
	assert!(v.is_array());
	assert!(v.as_array().unwrap().len() >= 10);
	aphrodite_hermes_free_string(json_ptr);
}
