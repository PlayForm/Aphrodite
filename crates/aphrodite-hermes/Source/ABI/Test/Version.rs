//! `aphrodite_hermes_version`: the crate reports a semver.

use std::ffi::CStr;

use crate::{aphrodite_hermes_free_string, aphrodite_hermes_version};

#[test]
fn test_version_is_semver() {
	let json_ptr = aphrodite_hermes_version();
	let json = unsafe { CStr::from_ptr(json_ptr) }.to_string_lossy().into_owned();
	let v:serde_json::Value = serde_json::from_str(&json).unwrap();
	let ver = v["version"].as_str().unwrap();
	assert!(
		ver.starts_with("0.") || ver.starts_with("1."),
		"expected semver starting with 0. or 1., got: {ver}"
	);
	aphrodite_hermes_free_string(json_ptr);
}
