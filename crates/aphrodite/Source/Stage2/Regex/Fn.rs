//! The declaration matcher.
//!
//! This is a hand-rolled prefix matcher, not a regex engine: the code reducer
//! only needs to answer "does this line start with one of the declaration
//! shapes", and the `Pattern` argument is used purely as a selector for which
//! check to run. Pulling in the `regex` crate for nine anchored prefix tests
//! would cost a compiled automaton per call for no expressive gain.
//!
//! The `Lower` argument is the caller's already-lowercased line: lowercasing
//! is the expensive part and the caller runs it once per line rather than once
//! per pattern (bug 18-P12).

/// Lightweight regex-like matching using simple prefix + word boundary checks.
pub fn RegexMatch(Line:&str, Lower:&str, Pattern:&str) -> Option<Vec<String>> {
	// `Lower` is `line.to_lowercase()` computed once by the caller (per line),
	// not per pattern, to avoid N allocations per line.

	if Pattern.contains(r"fn\s+(\w+)") {
		// Rust/Go function: "pub fn name" or "fn name" or "async fn name"
		if Lower.starts_with("fn ")
			|| Lower.starts_with("pub fn ")
			|| Lower.starts_with("async fn ")
			|| Lower.starts_with("pub async fn ")
		{
			return Some(vec![Line.to_string()]);
		}
	}
	if Pattern.contains(r"struct\s+(\w+)") && (Lower.starts_with("struct ") || Lower.starts_with("pub struct ")) {
		return Some(vec![Line.to_string()]);
	}
	if Pattern.contains(r"enum\s+(\w+)") && (Lower.starts_with("enum ") || Lower.starts_with("pub enum ")) {
		return Some(vec![Line.to_string()]);
	}
	if Pattern.contains(r"trait\s+(\w+)") && (Lower.starts_with("trait ") || Lower.starts_with("pub trait ")) {
		return Some(vec![Line.to_string()]);
	}
	if Pattern.contains(r"impl\b") && (Lower.starts_with("impl ") || Lower.starts_with("impl<")) {
		return Some(vec![Line.to_string()]);
	}
	if Pattern.contains(r"def\s+(\w+)\s*\(") && Lower.starts_with("def ") {
		return Some(vec![Line.to_string()]);
	}
	if Pattern.contains(r"class\s+(\w+)") && Lower.starts_with("class ") {
		return Some(vec![Line.to_string()]);
	}
	if Pattern.contains(r"func\s+(\w+)\s*\(") && Lower.starts_with("func ") {
		return Some(vec![Line.to_string()]);
	}
	if Pattern.contains(r"function\s+(\w+)")
		&& (Lower.starts_with("export function ")
			|| Lower.starts_with("export async function ")
			|| Lower.starts_with("function "))
	{
		return Some(vec![Line.to_string()]);
	}

	None
}
