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
fn Stamp() {
	let s = IsoTimestamp();
	// YYYY-MM-DDTHH:MM:SS.ffffffZ
	assert_eq!(s.len(), 27, "timestamp: {s}");
	assert!(s.ends_with('Z'));
	assert!(s.contains('T'));
	assert!(s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' && s.as_bytes()[10] == b'T');
}

#[test]
fn Quiet() {
	SetLevel(0);
	assert!(!Enabled());
	// level 0 must short-circuit before any file I/O: log_at(0, ...)
	// returns without touching the runtime home.
	LogAt(0, "test_off", &[("k", "v".to_string())]);
}

#[test]
fn Flatten() {
	assert_eq!(&*SanitizeKv("a\nb\r c"), "a b  c");
}

#[test]
fn Borrow() {
	// The common case allocates nothing: the sanitiser only builds a string when it
	// has a newline to replace.
	assert!(matches!(SanitizeKv("one line"), std::borrow::Cow::Borrowed(_)));
	assert!(matches!(SanitizeKv("two\nlines"), std::borrow::Cow::Owned(_)));
}

#[test]
fn Gate() {
	// The counter gate is what makes the instrumentation safe in a release build:
	// with the logger off, a bump must cost one atomic load and no write. Asserting
	// the snapshot is unchanged after a bump is the observable form of that.
	SetLevel(0);
	let Before = MetricsJSON();

	crate::Debug::Count::Bump::Fn::Fn(crate::Debug::Count::Enum::Enum::Warnings);
	crate::Debug::Timing::Add::Fn::Fn(crate::Debug::Timing::Enum::Enum::Render, 1_000);

	assert_eq!(
		MetricsJSON(),
		Before,
		"a disabled logger must not move a counter or a timing sample"
	);
}

#[test]
fn Reset() {
	SetLevel(0);
	let m = MetricsJSON();
	assert_eq!(m["markers_created"], 0);
	assert_eq!(m["retrieves"], 0);
}
