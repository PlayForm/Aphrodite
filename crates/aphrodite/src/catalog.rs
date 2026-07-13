//! Catalog generation - port of plugins/aphrodite/_hooks/catalog.py
//!
//! Builds compression catalog with by-type grouping, total savings,
//! conversation turns, referenced files. Supports toc (compact) and full modes.

use std::collections::HashMap;

use crate::{state::AphroditeState, struct_extract::floor_boundary};

/// Full catalog result with items, by_type grouping, stats.
pub fn build_catalog(state:&AphroditeState, mode:&str) -> serde_json::Value {
	let items:Vec<serde_json::Value> = state
		.recent_markers
		.iter()
		.map(|m| {
			if mode == "toc" {
				serde_json::json!({
					// Full 40-char hash: a truncated one is unresolvable via
					// exact-match retrieval (F3). Truncation is reserved for the
					// human-readable markdown table (format_catalog_table), not
					// machine-consumed JSON.
					"hash": &m.hash,
					"type": m.ccr_type,
					"size": m.size,
					"preview": floor_boundary(&m.preview, 120),
				})
			} else {
				serde_json::json!({
					"hash": m.hash,
					"type": m.ccr_type,
					"size": m.size,
					"preview": floor_boundary(&m.preview, 120),
					"turn": m.turn,
				})
			}
		})
		.collect();

	// Group by type
	let mut by_type:HashMap<String, Vec<String>> = HashMap::new();
	for m in &state.recent_markers {
		by_type.entry(m.ccr_type.clone()).or_default().push(m.hash.clone());
	}

	let by_type_json:serde_json::Map<String, serde_json::Value> = by_type
		.iter()
		.map(|(t, hashes)| {
			(
				t.clone(),
				serde_json::json!({
					"count": hashes.len(),
					"hashes": &hashes[..10.min(hashes.len())],
				}),
			)
		})
		.collect();

	// Savings per entry, not raw original size (report 05 F5): the marker
	// that replaces the content is what the agent actually receives, so
	// "saved" must subtract what still has to be sent. `MarkerEntry` doesn't
	// carry the exact rendered marker length, so the preview length (plus
	// its small fixed template overhead) is the best available stand-in -
	// this is still far closer to reality than reporting the full original
	// size as "saved", which is what a 100%-compression ratio would claim.
	let total_saved:usize = state
		.recent_markers
		.iter()
		.map(|m| m.size.saturating_sub(m.preview.len()))
		.sum();

	let mut result = serde_json::json!({
		"total_items": items.len(),
		"total_saved": total_saved,
		"total_saved_human": fmt_size(total_saved),
		"by_type": by_type_json,
		"items": items,
		"conv_turns": state.conv_index.len(),
		"referenced_files": state.referenced_files.len(),
	});

	// TOC mode adds retrieve recommendations
	if mode == "toc" && !items.is_empty() {
		let recommendations:Vec<String> = state
			.recent_markers
			.iter()
			.take(10)
			.map(|m| format!("{} ({}B) - {}", m.ccr_type, m.size, floor_boundary(&m.preview, 60)))
			.collect();
		result["recommendations"] = serde_json::json!(recommendations);
		result["hint"] = serde_json::json!("Use aphrodite_retrieve(hash) to expand any entry.");
	}

	result
}

/// Format catalog as markdown table (for LLM consumption).
pub fn format_catalog_table(state:&AphroditeState) -> String {
	let items = &state.recent_markers;
	if items.is_empty() {
		return "No compressed items yet.".to_string();
	}

	let mut lines = vec![
		format!(
			"Catalog: {} items {} saved {} turns {} files",
			items.len(),
			fmt_size(items.iter().map(|m| m.size).sum()),
			state.conv_index.len(),
			state.referenced_files.len(),
		),
		String::new(),
		"| Hash | Type | Size | Preview |".to_string(),
		"|------|------|------|---------|".to_string(),
	];

	for m in items.iter().rev().take(20) {
		let hash = &m.hash[..10.min(m.hash.len())];
		let size = fmt_size(m.size);
		let preview = m.preview.replace('|', "\\|");
		let preview = floor_boundary(&preview, 80);
		lines.push(format!("| {} | {} | {} | {} |", hash, m.ccr_type, size, preview));
	}

	lines.join("\n")
}

fn fmt_size(bytes:usize) -> String {
	if bytes >= 1024 * 1024 {
		format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
	} else if bytes >= 1024 {
		format!("{:.0}KB", bytes as f64 / 1024.0)
	} else {
		format!("{}B", bytes)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_empty_catalog() {
		let s = AphroditeState::default();
		let cat = build_catalog(&s, "full");
		assert_eq!(cat["total_items"], 0);
	}

	#[test]
	fn test_catalog_with_items() {
		let mut s = AphroditeState::default();
		s.record_marker(crate::state::MarkerEntry {
			hash:"abc123def456".into(),
			ccr_type:"code_rust".into(),
			size:1024,
			preview:"[code:3fns 42L]".into(),
			turn:1,
			center:None,
			meta:None,
		});
		s.record_marker(crate::state::MarkerEntry {
			hash:"def789abc012".into(),
			ccr_type:"diff".into(),
			size:2048,
			preview:"[diff:1F +2/-1 10L]".into(),
			turn:1,
			center:None,
			meta:None,
		});

		let cat = build_catalog(&s, "full");
		assert_eq!(cat["total_items"], 2);
		// T6 (F5): "saved" is size minus what still has to be sent (the
		// preview stands in for the rendered marker length) - NOT the raw
		// sum of original sizes, which is what pre-fix code reported here
		// (a flat 3072, i.e. claiming 100% "savings" that were never
		// realized).
		let expected = (1024 - "[code:3fns 42L]".len()) + (2048 - "[diff:1F +2/-1 10L]".len());
		assert_eq!(cat["total_saved"], expected);
	}

	#[test]
	fn test_toc_mode() {
		let mut s = AphroditeState::default();
		s.record_marker(crate::state::MarkerEntry {
			hash:"abc".into(),
			ccr_type:"text".into(),
			size:100,
			preview:"[text]".into(),
			turn:1,
			center:None,
			meta:None,
		});

		let cat = build_catalog(&s, "toc");
		assert!(cat["hint"].as_str().unwrap().contains("retrieve"));
	}

	#[test]
	fn test_empty_table() {
		let s = AphroditeState::default();
		assert_eq!(format_catalog_table(&s), "No compressed items yet.");
	}
}
