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
	Reducer::Type::Reducer,
};

/// Registry mapping CCR types to their reducer functions.
pub fn ReducerRegistry() -> HashMap<&'static str, Reducer> {
	let mut m:HashMap<&'static str, Reducer> = HashMap::new();
	m.insert("json", JSON::Fn::ReduceJSON);
	m.insert("json_list", JSON::Fn::ReduceJSON);
	m.insert("json_array", JSON::Fn::ReduceJSON);
	m.insert("diff", Diff::Fn::ReduceDiff);
	m.insert("git", Diff::Fn::ReduceDiff);
	m.insert("build_output", Build::Fn::ReduceBuild);
	m.insert("build_error", Build::Fn::ReduceBuild);
	m.insert("build", Build::Fn::ReduceBuild);
	m.insert("log", Build::Fn::ReduceBuild);
	m.insert("code_rust", Code::Fn::ReduceCode);
	m.insert("code_python", Code::Fn::ReduceCode);
	m.insert("code_go", Code::Fn::ReduceCode);
	m.insert("code_js", Code::Fn::ReduceCode);
	m.insert("code_ts", Code::Fn::ReduceCode);
	m.insert("code_sh", Code::Fn::ReduceCode);
	m.insert("code", Code::Fn::ReduceCode);
	m.insert("source_code", Code::Fn::ReduceCode);
	m
}
