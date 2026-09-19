//! Regression tests for the bench/corpus classifier finding: idiomatic Go
//! source (bench/corpus/code_go.go) was classified as `build` instead of
//! `source_code` because the case-insensitive `\bERROR\b` log pattern matched
//! the `error` in Go's `(string, error)` return idiom on nearly every
//! function, and log detection runs before code detection.
//!
//! The fix gates the ERROR/WARN log heuristics on log framing (`error[E0308]:`,
//! `warning:`, `[ERROR]`, `ValueError:`) or conventionally UPPERCASE levels,
//! so bare lowercase identifiers never trip log detection.

use std::path::PathBuf;

/// Absolute path to the bench corpus fixture, or `None` if this checkout has
/// no bench/corpus (packaged tarball) - tests then fall back to inline
/// content with the same shape, so they never hard-depend on repo layout.
fn corpus_fixture(name: &str) -> Option<String> {
	let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../bench/corpus").join(name);
	std::fs::read_to_string(p).ok()
}

fn go_source_with_error_returns() -> String {
	if let Some(content) = corpus_fixture("code_go.go") {
		return content;
	}
	// Inline fallback mirroring bench/corpus/code_go.go: many functions with
	// `(string, error)` returns (the old detector classified this as build).
	let mut content = String::from("package fixture\n\nimport \"fmt\"\n\n");
	for i in 0..8 {
		content.push_str(&format!(
			"type Record{i} struct {{\n\tID   int64\n\tName string\n\tTags []string\n}}\n\nfunc Process{i}(r \
			 *Record{i}, depth int) (string, error) {{\n\treturn fmt.Sprintf(\"%d-%d\", r.ID, depth+{i}), nil\n}}\n\n"
		));
	}
	content
}

#[test]
fn go_source_with_error_returns_is_source_code_not_build() {
	let content = go_source_with_error_returns();
	let ty = aphrodite::detect_type(&content);
	assert_eq!(
		ty, "source_code",
		"idiomatic Go with `(string, error)` returns must not be classified as build (found: {ty})"
	);
}

#[test]
fn framed_build_log_still_detects_as_build() {
	// Rust diagnostics keep their framing - real build logs must still route.
	let content = "\
   Compiling foo v0.1.0
error[E0308]: mismatched types
  --> src/lib.rs:90:17
warning: unused variable: `pending`
   Compiling bar v0.1.0
error: could not compile `foo`
";
	assert_eq!(aphrodite::detect_type(content), "build");
}

#[test]
fn bare_uppercase_error_level_detects_as_build() {
	// Conventionally UPPERCASE log levels still trip log detection.
	let content = "\
INFO starting build
WARN deprecated API used
ERROR compilation failed
FAILED test_x
PASSED test_y
";
	assert_eq!(aphrodite::detect_type(content), "build");
}

#[test]
fn bare_lowercase_error_identifier_is_not_build() {
	// A bare lowercase `error` identifier (no framing, no UPPERCASE level)
	// must not classify content as build output.
	let content = "\
function handle(data) {
    const error = validate(data);
    if (error) {
        return error.message;
    }
    return null;
}
";
	assert_ne!(aphrodite::detect_type(content), "build");
}

#[test]
fn real_build_log_fixture_still_detects_as_build() {
	// The corpus control fixture must keep its label after the gating change.
	if let Some(content) = corpus_fixture("build_log.txt") {
		assert_eq!(aphrodite::detect_type(&content), "build");
	}
}
