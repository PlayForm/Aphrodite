//! Registry-level tests: drift, the unknown-name path, and the file-backed tools.
//!
//! The consistency test is the important one - it is the only thing that catches a
//! schema added without a handler (an advertised tool that always errors) or a
//! handler added without a schema (a tool the model can never discover).

use crate::Tool::{Dispatch::Fn as dispatch, Registry};

#[test]
fn test_unknown_tool() {
	let r = dispatch("nonexistent", "{}");
	assert!(r["error"].as_str().unwrap().contains("unknown tool"));
}

// ── T11: schema/registry consistency ──────────────────────────
#[test]
fn schema_registry_names_match_tool_registry_keys() {
	// Catches classic registry/schema drift: every dispatchable tool must
	// have a schema, and vice versa (except `context_engine_pre_llm`,
	// which is an internal hook handler registered via
	// `ctx.register_context_engine` - never exposed as a callable tool
	// to Hermes, so it has no schema by design).
	let registry_names:std::collections::HashSet<&str> = Registry::Fn()
		.keys()
		.copied()
		.filter(|n| *n != "context_engine_pre_llm")
		.collect();
	let schema_names:std::collections::HashSet<String> = crate::Schema::All::Fn()
		.iter()
		.map(|s| s["name"].as_str().unwrap().to_string())
		.collect();

	for name in &registry_names {
		assert!(
			schema_names.contains(*name),
			"tool {name:?} is dispatchable but has no schema in Schema::All::Fn()"
		);
	}
	for name in &schema_names {
		assert!(
			registry_names.contains(name.as_str()),
			"schema {name:?} exists but is not dispatchable via tool_registry()"
		);
	}
}

#[test]
fn test_test_tool_roundtrips() {
	let _g = crate::test_guard();
	let r = dispatch("aphrodite_test", &serde_json::json!({"mode": "full"}).to_string());
	assert_eq!(r["status"], "ok", "smoke test should pass: {:?}", r);
	assert_eq!(r["passed"], r["total"]);
}

#[test]
fn test_prefetch_real_file() {
	let _g = crate::test_guard();
	let src = concat!(env!("CARGO_MANIFEST_DIR"), "/Source/Tool/mod.rs");
	let r = dispatch("aphrodite_prefetch", &serde_json::json!({"paths": [src]}).to_string());
	assert_eq!(r["loaded"], 1, "prefetch should load this source file: {:?}", r);
}
