//! The search-envelope shape: match rows, not a match count.
//!
//! Real Hermes `search_files` results ship either a `matches` array or a
//! path-grouped `matches_text` block, and the preview counts `path:line:content`
//! rows. Both forms are normalised to those rows here, which is why a 5,000-hit
//! search previews with the real count instead of `[search:20 hits ...]`.

use serde_json::{Map, Value};

/// Rebuild a search envelope's matches as grep-style `path:line:content` rows.
///
/// Returns `Some((body, ccr_type))` where `ccr_type` is `"search"` for real rows
/// and `"text"` for the count-only fallback (`"N total"`), or `None` when the
/// object is not a search envelope at all.
///
/// The distinction in the fallback is deliberate: a data object that merely
/// carries a `total_count` key (e.g. `{"total_count": 42, "items": [...]}`) is
/// NOT a search and must preview as itself, so only a genuine search shape
/// (`matches`/`matches_text`/`matches_format`/`truncated`) produces a label.
///
/// It applies no size cap: the rows feed only the preview and are never stored or
/// hashed, so truncating them would reintroduce the count that lied.
pub(crate) fn Fn(obj:&Map<String, Value>) -> Option<(String, &'static str)> {
	let count = obj.get("total_count").and_then(|c| c.as_u64())?;
	let truncated = obj.get("truncated").and_then(|t| t.as_bool()).unwrap_or(false);
	// ISSUE-11 WS2: build ONE grep-style line per REAL match and let
	// the search preview count them. The old take(20) cap made a
	// 5,000-hit search preview as "[search:20 hits ...]" - a count
	// that lied about the total.
	let mut lines:Vec<String> = Vec::new();
	if let Some(matches) = obj.get("matches").and_then(|m| m.as_array()) {
		for m in matches {
			if let (Some(p), Some(l)) = (m.get("path").or(m.get("file")).and_then(|v| v.as_str()), m.get("line")) {
				let content = m.get("content").and_then(|c| c.as_str()).unwrap_or("");
				lines.push(format!("{}:{}:{}", p, l, content));
			}
		}
	}
	// Real Hermes search_files results ship `matches_text` (path-
	// grouped: "path\n  <line>: <content>") instead of a `matches`
	// array. The old code ignored it, so every real search collapsed
	// to "[search:1L]" ("N total" has no file:line: rows for the
	// preview regex to count). Normalize it to the same grep-style
	// lines so the preview shows the real hit count and files.
	if lines.is_empty()
		&& let Some(text) = obj.get("matches_text").and_then(|t| t.as_str())
	{
		let mut cur_path:Option<&str> = None;
		for raw in text.lines() {
			let t = raw.trim();
			if t.is_empty() {
				continue;
			}
			if raw.trim_start().len() != raw.len() {
				// Indented row under the current path: "<line>: <content>".
				if let (Some(p), Some((lno, rest))) = (cur_path, t.split_once(':')) {
					lines.push(format!("{}:{}:{}", p, lno.trim(), rest.trim_start()));
				}
			} else {
				cur_path = Some(t);
			}
		}
	}
	if lines.is_empty() {
		// No per-match content to count. Surface the REAL total and
		// the truncated flag as the payload itself, typed `text` so
		// the generic preview arm renders "0 total" / "N total
		// (truncated)" instead of an unreadable "[search:1L]".
		let is_search_shape = obj.get("matches").is_some()
			|| obj.get("matches_text").is_some()
			|| obj.get("matches_format").is_some()
			|| truncated;
		if !is_search_shape {
			return None;
		}
		let label = if truncated {
			format!("{} total (truncated)", count)
		} else {
			format!("{} total", count)
		};
		Some((label, "text"))
	} else {
		Some((lines.join("\n"), "search"))
	}
}
