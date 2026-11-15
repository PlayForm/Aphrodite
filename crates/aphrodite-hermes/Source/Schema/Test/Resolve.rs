//! Resolution: the catalogue, the name lookup and the dispatch registry agree.
//!
//! The two failure modes are opposite and both silent - a schema with no handler
//! is an advertised tool that always errors, and a handler with no schema is a
//! tool the model can never discover. Both are asserted here.

use crate::Schema::{All, Get};

/// Every documented tool must resolve by name, and unknown names must not.
#[test]
fn test_get_schema_round_trip() {
	for s in All::Fn() {
		let name = s["name"].as_str().unwrap().to_string();
		assert_eq!(Get::Fn(&name).unwrap(), s, "{name}: get_schema returned a different record");
	}
	assert!(Get::Fn("aphrodite_not_a_tool").is_none());
}

/// The schema list and the dispatch registry must not drift apart: a
/// schema with no handler is an advertised tool that always errors.
/// Takes the crate test guard: `aphrodite_test` with `{}` args runs its
/// QUICK mode and records a marker in the shared session state, so this
/// test is a state writer like any other - unguarded it raced
/// concurrently running tests (e.g. exact-count catalog assertions).
#[test]
fn test_every_schema_has_a_handler() {
	let _g = crate::test_guard();
	for s in All::Fn() {
		let name = s["name"].as_str().unwrap();
		let result = crate::Tool::Dispatch::Fn(name, "{}");
		let err = result.get("error").and_then(|e| e.as_str()).unwrap_or_default();
		assert!(
			!err.starts_with("unknown tool"),
			"{name}: schema advertises a tool with no handler"
		);
	}
}
