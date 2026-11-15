//! The catalogue's tool count and membership.
//!
//! This is the one place a build profile changes observable output, so it is
//! asserted in both directions: a dev build must advertise the dev-only tool, and
//! a release build must NOT.

use crate::Schema::All;

/// Catalog contract: the manifest declares 13 production tools; a release
/// dylib (debug_assertions off) must register exactly those 13, and a dev
/// build (debug_assertions on) adds the dev-only `aphrodite_debug` (14).
/// This is what `hermes plugins validate` rule 6 enforces - a release
/// dylib registering `aphrodite_debug` fails catalog admission as an
/// undeclared tool (teknium review, PR 118488).
#[test]
fn test_catalog_tool_count_matches_manifest() {
	let names:Vec<String> = All::Fn()
		.into_iter()
		.map(|s| s["name"].as_str().expect("name must be a string").to_string())
		.collect();
	let has_debug = names.iter().any(|n| n == "aphrodite_debug");
	let production = [
		"aphrodite_compress",
		"aphrodite_retrieve",
		"aphrodite_stats",
		"aphrodite_files",
		"aphrodite_diff",
		"aphrodite_search",
		"aphrodite_directive",
		"aphrodite_test",
		"aphrodite_catalog",
		"aphrodite_reclassify",
		"aphrodite_prefetch",
		"aphrodite_prefetch_status",
		"aphrodite_rebuild",
	];
	for name in production {
		assert!(names.iter().any(|n| n == name), "missing production tool {name}");
	}
	#[cfg(debug_assertions)]
	{
		assert_eq!(names.len(), 14, "dev build must register 14 tools (13 + aphrodite_debug)");
		assert!(has_debug, "dev build must register aphrodite_debug");
	}
	#[cfg(not(debug_assertions))]
	{
		assert_eq!(names.len(), 13, "release build must register exactly the 13 declared tools");
		assert!(!has_debug, "release build must NOT register dev-only aphrodite_debug");
	}
}
