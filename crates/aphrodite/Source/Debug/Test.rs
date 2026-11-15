//! The Debug test battery: the timestamp shape, the level gate, the kv
//! sanitiser and the counter snapshot.
//!
//! Every test drives the logger through its public gate. Nothing here asserts
//! on file contents: the log destination is a per-session path under the
//! runtime home, and a test that wrote there would be writing into the user's
//! own history.

use crate::Debug::{
	Level::{Enabled::Fn::Fn as Enabled, Set::Fn::Fn as SetLevel},
	Log::{At::Fn::Fn as LogAt, Sanitize::Fn::Fn as SanitizeKv},
	Metrics::Fn::Fn as MetricsJSON,
	Time::Fn::Fn as IsoTimestamp,
};

#[test]
fn TestIsoTimestampShape() {
	let s = IsoTimestamp();
	// YYYY-MM-DDTHH:MM:SS.ffffffZ
	assert_eq!(s.len(), 27, "timestamp: {s}");
	assert!(s.ends_with('Z'));
	assert!(s.contains('T'));
	assert!(s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' && s.as_bytes()[10] == b'T');
}

#[test]
fn TestLevelGateOffIsQuiet() {
	SetLevel(0);
	assert!(!Enabled());
	// level 0 must short-circuit before any file I/O: log_at(0, ...)
	// returns without touching the runtime home.
	LogAt(0, "test_off", &[("k", "v".to_string())]);
}

#[test]
fn TestSanitizeKvFlattensNewlines() {
	assert_eq!(SanitizeKv("a\nb\r c"), "a b  c");
}

#[test]
fn TestCountersResetByDefault() {
	SetLevel(0);
	let m = MetricsJSON();
	assert_eq!(m["markers_created"], 0);
	assert_eq!(m["retrieves"], 0);
}
