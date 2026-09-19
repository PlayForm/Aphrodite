//! Stage 2 compression - semantic reduction of CCR-stored content.
//! Port of plugins/aphrodite/_stage2.py
//!
//! Produces a denser version of a piece of content on demand via the
//! stateless `aphrodite_stage2(content, ccr_type)` ABI (see
//! `lib.rs::aphrodite_stage2`) - it takes content directly and returns the
//! reduced string, with no interaction with the CCR store or the session's
//! inline store.
//!
//! There is currently no depth-aware retrieval: no `aphrodite_retrieve` /
//! `resolve_one` / `resolve::expand` entry point accepts a `depth` parameter,
//! and nothing in this crate writes a stage-2-reduced value back into the
//! store under a `{hash}#stage2` key for a later plain retrieval to pick up
//! (report 05 F6 - a version of this docstring used to claim
//! `aphrodite_retrieve(hash, depth=2)` returns the reduced version; that
//! entry point does not exist). Wiring depth-aware retrieval up for real is
//! a deliberate feature decision - see `.plans/05-compression-pipeline.md`
//! §5 - not something this module does today.
//!
//! Reducers per content type:
//! - JSON: whitespace minification + key extraction
//! - Build/log: error/warning/summary extraction
//! - Diff: file-level summary with hunk counts
//! - Code: function/struct/class signature extraction
//! - Other: pass through (return None = no reduction)

use std::collections::HashMap;

/// Minimum content size before attempting reduction.
const MIN_STAGE2_SIZE: usize = 80;

type Reducer = fn(&str) -> Option<String>;

/// Reduce JSON content: minify + extract structural keys.
fn reduce_json(content: &str) -> Option<String> {
	let data: serde_json::Value = match serde_json::from_str(content) {
		Ok(v) => v,
		Err(_) => return None,
	};
	let structural = match &data {
		serde_json::Value::Object(map) => {
			let keys: Vec<&str> = map.keys().map(|k| k.as_str()).take(10).collect();
			format!("[json:{} keys: {}]", map.len(), keys.join(", "))
		},
		serde_json::Value::Array(arr) => {
			let mut s = format!("[json_list:{} items]", arr.len());
			if let Some(first) = arr.first() {
				if let Some(obj) = first.as_object() {
					let keys: Vec<&str> = obj.keys().map(|k| k.as_str()).take(10).collect();
					s.push_str(&format!(" schema: {}", keys.join(", ")));
				}
			}
			s
		},
		_ => format!("[json:{}]", type_name(&data)),
	};
	// Minify
	let minified = serde_json::to_string(&data).unwrap_or_default();
	Some(format!("{}\n{}", structural, minified))
}

fn type_name(v: &serde_json::Value) -> &'static str {
	match v {
		serde_json::Value::Null => "null",
		serde_json::Value::Bool(_) => "bool",
		serde_json::Value::Number(_) => "number",
		serde_json::Value::String(_) => "string",
		serde_json::Value::Array(_) => "array",
		serde_json::Value::Object(_) => "object",
	}
}

/// Reduce build/log output: extract errors, warnings, summaries.
fn reduce_build(content: &str) -> Option<String> {
	let mut errors: Vec<String> = Vec::new();
	let mut warnings: Vec<String> = Vec::new();
	let mut summary_lines: Vec<String> = Vec::new();

	let summary_kw = ["compiling", "finished", "running", "test result", "passed", "failed"];

	for line in content.lines() {
		let stripped = line.trim();
		if stripped.is_empty() {
			continue;
		}
		let lower = stripped.to_lowercase();

		if lower.contains("error") && !lower.starts_with("error:") {
			errors.push(stripped.chars().take(200).collect());
		} else if lower.contains("warning") && !lower.starts_with("warning:") {
			warnings.push(stripped.chars().take(200).collect());
		} else if summary_kw.iter().any(|kw| lower.contains(kw)) {
			summary_lines.push(stripped.chars().take(200).collect());
		}
	}

	let mut parts: Vec<String> = Vec::new();
	if !summary_lines.is_empty() {
		parts.push("[build summary]".into());
		parts.extend(summary_lines.into_iter().take(10));
	}
	if !errors.is_empty() {
		parts.push(format!("[{} errors]", errors.len()));
		parts.extend(errors.into_iter().take(20));
	}
	if !warnings.is_empty() {
		parts.push(format!("[{} warnings]", warnings.len()));
		parts.extend(warnings.into_iter().take(10));
	}

	if parts.is_empty() { None } else { Some(parts.join("\n")) }
}

/// Reduce diff content: file-level summary with hunk counts.
fn reduce_diff(content: &str) -> Option<String> {
	let mut files: Vec<(String, usize)> = Vec::new();
	let mut current_file: Option<String> = None;
	let mut hunks: usize = 0;

	for line in content.lines() {
		if line.starts_with("diff --git ") {
			if let Some(f) = current_file.take() {
				files.push((f, hunks));
			}
			current_file = Some(line.to_string());
			hunks = 0;
		} else if line.starts_with("@@ ") {
			hunks += 1;
		}
	}
	if let Some(f) = current_file {
		files.push((f, hunks));
	}

	if files.is_empty() {
		return None;
	}

	let mut parts = vec![format!("[diff:{} files]", files.len())];
	for (fname, hcount) in &files {
		parts.push(format!("  {} ({} hunks)", fname, hcount));
	}
	Some(parts.join("\n"))
}

/// Reduce code: extract function/struct/class signatures.
fn reduce_code(content: &str) -> Option<String> {
	#[derive(Debug)]
	struct SigPattern {
		kind: &'static str,
		pattern: &'static str,
	}

	let patterns = [
		SigPattern { kind: "fn", pattern: r"^(pub\s+)?(async\s+)?fn\s+(\w+)" },
		SigPattern { kind: "struct", pattern: r"^(pub\s+)?struct\s+(\w+)" },
		SigPattern { kind: "enum", pattern: r"^(pub\s+)?enum\s+(\w+)" },
		SigPattern { kind: "trait", pattern: r"^(pub\s+)?trait\s+(\w+)" },
		SigPattern { kind: "impl", pattern: r"^(pub\s+)?impl\b" },
		SigPattern { kind: "def", pattern: r"^def\s+(\w+)\s*\(" },
		SigPattern { kind: "class", pattern: r"^class\s+(\w+)" },
		SigPattern { kind: "func", pattern: r"^func\s+(\w+)\s*\(" },
		SigPattern { kind: "function", pattern: r"^export\s+(?:async\s+)?function\s+(\w+)" },
	];

	let mut sigs: Vec<String> = Vec::new();
	for line in content.lines() {
		let trimmed = line.trim();
		// Lowercase ONCE per line (was recomputed inside regex_match for
		// every one of the 9 patterns → 9 allocs/line). Bug 18-P12.
		let lower = trimmed.to_lowercase();
		for sp in &patterns {
			if let Some(_caps) = regex_match(trimmed, &lower, sp.pattern) {
				let sig_line: String = trimmed.chars().take(120).collect();
				sigs.push(format!("  [{}] {}", sp.kind, sig_line));
			}
		}
	}

	if sigs.is_empty() {
		None
	} else {
		let mut result = String::from("[code structure]");
		for sig in sigs.iter().take(50) {
			result.push('\n');
			result.push_str(sig);
		}
		Some(result)
	}
}

/// Lightweight regex-like matching using simple prefix + word boundary checks.
fn regex_match(line: &str, lower: &str, pattern: &str) -> Option<Vec<String>> {
	// `lower` is `line.to_lowercase()` computed once by the caller (per line),
	// not per pattern, to avoid N allocations per line.

	if pattern.contains(r"fn\s+(\w+)") {
		// Rust/Go function: "pub fn name" or "fn name" or "async fn name"
		if lower.starts_with("fn ")
			|| lower.starts_with("pub fn ")
			|| lower.starts_with("async fn ")
			|| lower.starts_with("pub async fn ")
		{
			return Some(vec![line.to_string()]);
		}
	}
	if pattern.contains(r"struct\s+(\w+)") && (lower.starts_with("struct ") || lower.starts_with("pub struct ")) {
		return Some(vec![line.to_string()]);
	}
	if pattern.contains(r"enum\s+(\w+)") && (lower.starts_with("enum ") || lower.starts_with("pub enum ")) {
		return Some(vec![line.to_string()]);
	}
	if pattern.contains(r"trait\s+(\w+)") && (lower.starts_with("trait ") || lower.starts_with("pub trait ")) {
		return Some(vec![line.to_string()]);
	}
	if pattern.contains(r"impl\b") && (lower.starts_with("impl ") || lower.starts_with("impl<")) {
		return Some(vec![line.to_string()]);
	}
	if pattern.contains(r"def\s+(\w+)\s*\(") && lower.starts_with("def ") {
		return Some(vec![line.to_string()]);
	}
	if pattern.contains(r"class\s+(\w+)") && lower.starts_with("class ") {
		return Some(vec![line.to_string()]);
	}
	if pattern.contains(r"func\s+(\w+)\s*\(") && lower.starts_with("func ") {
		return Some(vec![line.to_string()]);
	}
	if pattern.contains(r"function\s+(\w+)")
		&& (lower.starts_with("export function ")
			|| lower.starts_with("export async function ")
			|| lower.starts_with("function "))
	{
		return Some(vec![line.to_string()]);
	}

	None
}

/// Registry mapping CCR types to their reducer functions.
fn reducer_registry() -> HashMap<&'static str, Reducer> {
	let mut m: HashMap<&'static str, Reducer> = HashMap::new();
	m.insert("json", reduce_json);
	m.insert("json_list", reduce_json);
	m.insert("json_array", reduce_json);
	m.insert("diff", reduce_diff);
	m.insert("git", reduce_diff);
	m.insert("build_output", reduce_build);
	m.insert("build_error", reduce_build);
	m.insert("build", reduce_build);
	m.insert("log", reduce_build);
	m.insert("code_rust", reduce_code);
	m.insert("code_python", reduce_code);
	m.insert("code_go", reduce_code);
	m.insert("code_js", reduce_code);
	m.insert("code_ts", reduce_code);
	m.insert("code_sh", reduce_code);
	m.insert("code", reduce_code);
	m.insert("source_code", reduce_code);
	m
}

/// Produce a semantically reduced version of content.
///
/// Returns `Some(reduced_string)` if reduction was beneficial,
/// or `None` if the content is too small, no reducer exists for the type,
/// or the reducer produced no savings.
pub fn compress_stage2(content: &str, ccr_type: &str) -> Option<String> {
	if content.len() < MIN_STAGE2_SIZE {
		return None;
	}

	let registry = reducer_registry();
	let reducer = registry.get(ccr_type)?;

	let reduced = reducer(content)?;

	// Guard: don't return if reduction didn't help
	if reduced == content || reduced.len() >= content.len() {
		return None;
	}

	Some(reduced)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_small_content_returns_none() {
		assert_eq!(compress_stage2("short", "code_rust"), None);
	}

	#[test]
	fn test_unknown_type_returns_none() {
		let content = "a".repeat(100);
		assert_eq!(compress_stage2(&content, "unknown_type"), None);
	}

	#[test]
	fn test_reduce_json_object() {
		let json = r#"{"name": "Alice", "age": 30, "city": "NYC", "role": "admin"}"#;
		let result = reduce_json(json);
		assert!(result.is_some());
		let r = result.unwrap();
		assert!(r.contains("[json:4 keys:"));
		assert!(r.contains("name"));
	}

	#[test]
	fn test_reduce_json_invalid() {
		assert_eq!(reduce_json("not json at all"), None);
	}

	#[test]
	fn test_reduce_build_errors() {
		// Use mid-line error/warning (prefix "error:" and "warning:" are intentionally
		// excluded because they're structured prefixes handled by summary extraction)
		let build = "   Compiling foo v0.1.0\n   process failed with error code 1\n   found a warning in the \
		             pipeline\n   Finished dev [unoptimized] target(s)\n";
		let result = reduce_build(build);
		assert!(result.is_some());
		let r = result.unwrap();
		assert!(r.contains("[build summary]"));
		assert!(r.contains("[1 errors]"));
		assert!(r.contains("[1 warnings]"));
	}

	#[test]
	fn test_reduce_build_empty() {
		assert_eq!(reduce_build(""), None);
	}

	#[test]
	fn test_reduce_diff() {
		let diff = "diff --git a/src/main.rs b/src/main.rs\n@@ -1,5 +1,7 @@\n+added line\n-old line\n@@ -10,3 +12,4 \
		            @@\ndiff --git a/Cargo.toml b/Cargo.toml\n@@ -1,1 +1,1 @@\n";
		let result = reduce_diff(diff);
		assert!(result.is_some());
		let r = result.unwrap();
		assert!(r.contains("[diff:2 files]"));
		assert!(r.contains("main.rs"));
		assert!(r.contains("Cargo.toml"));
	}

	#[test]
	fn test_reduce_code_rust() {
		let code =
			"pub fn main() {\n    println!(\"hello\");\n}\n\npub struct Foo {\n    x: i32,\n}\n\nfn helper() {}\n";
		let result = reduce_code(code);
		assert!(result.is_some());
		let r = result.unwrap();
		assert!(r.contains("[code structure]"));
		assert!(r.contains("[fn]"));
		assert!(r.contains("[struct]"));
	}

	#[test]
	fn test_integration_json_minifies() {
		// Heavily whitespace-padded JSON (many keys, deep indentation) so that
		// minification + the structural header is still smaller than the
		// pretty-printed original - a real size assertion, not just "no panic".
		let content = "{\n  \"users\": [\n    {\"id\": 1, \"name\": \"Alice\", \"active\": true, \"role\": \
		               \"admin\"},\n    {\"id\": 2, \"name\": \"Bob\", \"active\": false, \"role\": \"user\"},\n    \
		               {\"id\": 3, \"name\": \"Carol\", \"active\": true, \"role\": \"user\"}\n  ],\n  \"total\": \
		               3,\n  \"page\": 1,\n  \"per_page\": 10\n}"
			.to_string();
		let original_len = content.len();
		let reduced = reduce_json(&content);
		assert!(reduced.is_some(), "reduce_json returned None");
		let r = reduced.unwrap();
		assert!(r.starts_with("[json:4 keys:"));
		assert!(r.contains("\"users\""));
		// The whole point of stage2 reduction is to save space - assert it
		// actually does, not merely that it runs without panicking.
		assert!(
			r.len() < original_len,
			"reduced output ({} bytes) should be smaller than the pretty-printed original ({} bytes)",
			r.len(),
			original_len
		);
	}

	#[test]
	fn test_stage2_skips_small() {
		assert_eq!(compress_stage2(r#"{"a":1}"#, "json"), None);
	}

	#[test]
	fn test_integration_build_stage2() {
		let content = "   Compiling foo v0.1.0\n   error: something broke\n   ".to_string() + &"x".repeat(80);
		let result = compress_stage2(&content, "build_error");
		assert!(result.is_some());
		assert!(result.unwrap().len() < content.len());
	}
}
