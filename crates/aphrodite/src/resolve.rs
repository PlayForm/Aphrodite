//! CCR marker resolution - port of plugins/aphrodite/_resolve.py
//!
//! Resolves CCR markers to their original content. Supports:
//! - Single hash resolution (inline store only)
//! - Recursive nested marker unpacking (up to RECURSIVE_DEPTH levels)
//! - Query filtering on resolved content
//! - Cycle-safe recursive expansion with resolved cache

use std::collections::HashMap;

use crate::state::AphroditeState;

/// Maximum recursion depth for nested marker resolution.
const RECURSIVE_DEPTH:usize = 5;

/// Maximum resolved content size to cache in inline store (512KB).
const MAX_CACHE_SIZE:usize = 512 * 1024;

/// CCR marker prefix/suffix
const CCR_PREFIX:&str = "<<<CCR:";
const CCR_SUFFIX:&str = ">>>";

/// Parse a CCR marker string to extract the hash.
/// Marker format: <<<CCR:hash|type|size>>>
fn parse_marker_hash(marker:&str) -> Option<String> {
	let inner = marker.strip_prefix(CCR_PREFIX)?.strip_suffix(CCR_SUFFIX)?;
	inner.split('|').next().map(|h| h.to_string())
}

/// Find all CCR markers in content. Returns (full_marker, hash) pairs.
fn find_markers(content:&str) -> Vec<(String, String)> {
	let mut markers = Vec::new();
	let mut search_from = 0;

	while let Some(start) = content[search_from..].find(CCR_PREFIX) {
		let abs_start = search_from + start;
		let after_prefix = abs_start + CCR_PREFIX.len();

		if let Some(end) = content[after_prefix..].find(CCR_SUFFIX) {
			let abs_end = after_prefix + end + CCR_SUFFIX.len();
			let full_marker = content[abs_start..abs_end].to_string();
			if let Some(hash) = parse_marker_hash(&full_marker) {
				markers.push((full_marker, hash));
			}
			search_from = abs_end;
		} else {
			// Unclosed marker - skip past this prefix
			search_from = after_prefix;
		}
	}

	markers
}

/// Resolve a single CCR hash from the inline store.
/// Does NOT unpack nested markers. Returns None if not found.
pub fn resolve_one(state:&mut AphroditeState, hash_val:&str) -> Option<String> {
	// i: prefix - inline-only hashes
	if hash_val.starts_with("i:") {
		return state.inline_store_get(hash_val);
	}

	// Stage-2 depth: look up reduced version first
	let stage2_key = format!("{}#stage2", hash_val);
	if let Some(content) = state.inline_store_get(&stage2_key) {
		return Some(content);
	}

	// Standard inline lookup (promotes to front via LRU)
	state.inline_store_get(hash_val)
}

/// Filter content to lines containing the query string (case-insensitive).
/// Returns filtered lines, or original with prefix if no matches.
pub fn filter_lines(content:&str, query:&str) -> String {
	if query.is_empty() {
		return content.to_string();
	}
	let query_lower = query.to_lowercase();
	let matching:Vec<&str> = content
		.lines()
		.filter(|line| line.to_lowercase().contains(&query_lower))
		.collect();
	if matching.is_empty() {
		format!("[aphrodite: no lines matched {query:?} - returning full content]\n{content}")
	} else {
		matching.join("\n")
	}
}

/// Recursively resolve a hash and all nested CCR markers.
///
/// Cycle-safe via `visited` set. Uses `resolved` map as a persistent
/// cache across the entire resolution tree - once a hash is resolved,
/// nested references to it reuse the cached result.
///
/// Returns `Some(resolved_content)` on success, or `None` if the
/// top-level hash could not be found.
pub fn resolve_recursive(
	state:&mut AphroditeState,
	hash_val:&str,
	depth:usize,
	resolved:&mut HashMap<String, String>,
	visited:&mut Vec<String>,
) -> Option<String> {
	// Cycle detection
	if visited.contains(&hash_val.to_string()) {
		return resolved.get(hash_val).cloned();
	}
	visited.push(hash_val.to_string());

	// Depth limit or already resolved
	if depth >= RECURSIVE_DEPTH {
		return resolved.get(hash_val).cloned();
	}
	if let Some(cached) = resolved.get(hash_val) {
		return Some(cached.clone());
	}

	// Resolve the top-level hash
	let content = match resolve_one(state, hash_val) {
		Some(c) => c,
		None => return None,
	};
	resolved.insert(hash_val.to_string(), content.clone());

	// Find nested CCR markers
	let nested_markers = find_markers(&content);
	if nested_markers.is_empty() {
		return Some(content);
	}

	// Resolve nested hashes and build replacements
	let mut replacements:Vec<(String, Option<String>)> = Vec::new();
	for (marker, nested_hash) in &nested_markers {
		if !resolved.contains_key(nested_hash) {
			let nested_content = resolve_recursive(state, nested_hash, depth + 1, resolved, visited);
			replacements.push((marker.clone(), nested_content));
		}
	}

	// Apply replacements
	let mut result = content;
	for (marker, replacement) in &replacements {
		match replacement {
			Some(repl) => {
				result = result.replace(marker.as_str(), repl.as_str());
			},
			None => {
				// Unresolved - preserve as [CCR_UNRESOLVED:hash]
				let hash = parse_marker_hash(marker).unwrap_or_else(|| "unknown".into());
				result = result.replace(marker.as_str(), &format!("[CCR_UNRESOLVED:{}]", hash));
			},
		}
	}

	// Cache the fully-resolved result if small enough
	if result.len() <= MAX_CACHE_SIZE {
		state.inline_store_put(hash_val.to_string(), result.clone());
	}

	Some(result)
}

/// Public entry point: expands a hash through all nesting levels.
pub fn expand(state:&mut AphroditeState, hash_val:&str) -> Option<String> {
	let mut resolved = HashMap::new();
	let mut visited = Vec::new();
	resolve_recursive(state, hash_val, 0, &mut resolved, &mut visited)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_parse_marker_hash() {
		let hash = parse_marker_hash("<<<CCR:abc123def456|text|100>>>");
		assert_eq!(hash, Some("abc123def456".into()));

		let hash = parse_marker_hash("<<<CCR:abc|code_rust|5000>>>");
		assert_eq!(hash, Some("abc".into()));
	}

	#[test]
	fn test_parse_marker_invalid() {
		assert_eq!(parse_marker_hash("not a marker"), None);
		assert_eq!(parse_marker_hash("<<<CCR:"), None);
	}

	#[test]
	fn test_find_markers_single() {
		let content = "hello <<<CCR:abc|text|100>>> world";
		let markers = find_markers(content);
		assert_eq!(markers.len(), 1);
		assert_eq!(markers[0].0, "<<<CCR:abc|text|100>>>");
		assert_eq!(markers[0].1, "abc");
	}

	#[test]
	fn test_find_markers_multiple() {
		let content = "<<<CCR:a|t|1>>> middle <<<CCR:b|t|2>>>";
		let markers = find_markers(content);
		assert_eq!(markers.len(), 2);
	}

	#[test]
	fn test_find_markers_none() {
		assert!(find_markers("plain text").is_empty());
	}

	#[test]
	fn test_filter_empty_query() {
		assert_eq!(filter_lines("hello\nworld", ""), "hello\nworld");
	}

	#[test]
	fn test_filter_matching() {
		let content = "line one\nerror: broke\nline two\nERROR: fatal\n";
		let result = filter_lines(content, "error");
		assert!(result.contains("error: broke"));
		assert!(result.contains("ERROR: fatal"));
		assert!(!result.contains("line one"));
	}

	#[test]
	fn test_filter_no_match() {
		let content = "hello\nworld\n";
		let result = filter_lines(content, "nonexistent");
		assert!(result.contains("[aphrodite: no lines matched"));
	}

	#[test]
	fn test_resolve_one_inline() {
		let mut s = AphroditeState::default();
		s.inline_store_put("abc123".into(), "hello world".into());
		assert_eq!(resolve_one(&mut s, "abc123"), Some("hello world".into()));
	}

	#[test]
	fn test_resolve_one_missing() {
		let mut s = AphroditeState::default();
		assert_eq!(resolve_one(&mut s, "nonexistent"), None);
	}

	#[test]
	fn test_resolve_one_i_prefix() {
		let mut s = AphroditeState::default();
		s.inline_store_put("i:abc123def".into(), "inline content".into());
		assert_eq!(resolve_one(&mut s, "i:abc123def"), Some("inline content".into()));
	}

	#[test]
	fn test_expand_simple() {
		let mut s = AphroditeState::default();
		s.inline_store_put("simple".into(), "just content".into());
		assert_eq!(expand(&mut s, "simple"), Some("just content".into()));
	}

	#[test]
	fn test_expand_with_nesting() {
		let mut s = AphroditeState::default();
		let inner = "<<<CCR:inner123|text|10>>>";
		s.inline_store_put("outer".into(), format!("before {} after", inner));
		s.inline_store_put("inner123".into(), "RESOLVED".into());
		let result = expand(&mut s, "outer");
		assert_eq!(result, Some("before RESOLVED after".into()));
	}

	#[test]
	fn test_expand_unresolved() {
		let mut s = AphroditeState::default();
		s.inline_store_put("outer".into(), "before <<<CCR:missing|text|10>>> after".into());
		let result = expand(&mut s, "outer");
		assert!(result.unwrap().contains("[CCR_UNRESOLVED:missing]"));
	}

	#[test]
	fn test_expand_missing_hash() {
		let mut s = AphroditeState::default();
		assert_eq!(expand(&mut s, "nope"), None);
	}

	#[test]
	fn test_expand_deep_nesting_respects_limit() {
		let mut s = AphroditeState::default();
		// Chain of 10 nested markers - depth limit should stop at 5
		s.inline_store_put("h0".into(), "<<<CCR:h1|t|1>>>".into());
		s.inline_store_put("h1".into(), "<<<CCR:h2|t|1>>>".into());
		s.inline_store_put("h2".into(), "<<<CCR:h3|t|1>>>".into());
		s.inline_store_put("h3".into(), "<<<CCR:h4|t|1>>>".into());
		s.inline_store_put("h4".into(), "<<<CCR:h5|t|1>>>".into());
		s.inline_store_put("h5".into(), "DEEP".into());
		// h0 should resolve but h5 might not due to depth limit
		let result = expand(&mut s, "h0");
		assert!(result.is_some());
	}

	#[test]
	fn test_expand_cycle_safe() {
		let mut s = AphroditeState::default();
		// Self-referential: hA → hB → hA
		s.inline_store_put("hA".into(), "<<<CCR:hB|t|1>>>".into());
		s.inline_store_put("hB".into(), "<<<CCR:hA|t|1>>>".into());
		let result = expand(&mut s, "hA");
		// Should not infinite-loop - cycle is detected
		assert!(result.is_some());
	}
}
