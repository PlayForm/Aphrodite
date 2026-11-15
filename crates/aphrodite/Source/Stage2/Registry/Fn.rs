//! The CCR-type → reducer registry.
//!
//! The registry is where several CCR type strings are allowed to share one
//! reducer (every `code_*` family is the same signature scan). It is rebuilt
//! per call on purpose: the map is small, immutable in practice, and a
//! process-global `LazyLock` would have to be reasoned about across the dylib
//! hot-reload path, where statics outlive the code that built them.
//!
//! A type that is absent from the registry is the "no reducer" answer - that
//! is what makes stage-2 a no-op for previews, listings and everything else
//! whose bytes are already dense.

use std::collections::HashMap;

use crate::Stage2::{
	Reduce::{Build, Code, Diff, JSON},
	Reducer::Reducer,
};

/// Registry mapping CCR types to their reducer functions.
pub fn ReducerRegistry() -> HashMap<&'static str, Reducer> {
	let mut m:HashMap<&'static str, Reducer> = HashMap::new();
	m.insert("json", JSON::ReduceJSON);
	m.insert("json_list", JSON::ReduceJSON);
	m.insert("json_array", JSON::ReduceJSON);
	m.insert("diff", Diff::ReduceDiff);
	m.insert("git", Diff::ReduceDiff);
	m.insert("build_output", Build::ReduceBuild);
	m.insert("build_error", Build::ReduceBuild);
	m.insert("build", Build::ReduceBuild);
	m.insert("log", Build::ReduceBuild);
	m.insert("code_rust", Code::ReduceCode);
	m.insert("code_python", Code::ReduceCode);
	m.insert("code_go", Code::ReduceCode);
	m.insert("code_js", Code::ReduceCode);
	m.insert("code_ts", Code::ReduceCode);
	m.insert("code_sh", Code::ReduceCode);
	m.insert("code", Code::ReduceCode);
	m.insert("source_code", Code::ReduceCode);
	m
}
