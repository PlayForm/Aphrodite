//! `CCRMarker` - the public entry point.
//!
//! Everything that makes a preview safe to emit happens here, before the
//! grammar is chosen: newlines become spaces (a marker must stay on its own
//! line, and a multi-line preview would break every parser that reads the next
//! line as the preview line), control characters are dropped, and an optional
//! headroom budget truncates by char count rather than byte count.
//!
//! Metadata is sanitized separately and capped at 300 bytes: `|` becomes `/`
//! so a value can never be mistaken for a field separator, and the truncation
//! goes through `StructExtract::Format::Floor::Fn` so a multi-byte character is
//! never split in half.

use std::collections::HashMap;

use super::Block::RenderMarkerBlock;

/// Build a CCR output block.
///
/// - `HashValue`: the content hash
/// - `CCRType`: content type string (e.g. "code_rust", "build")
/// - `Size`: original content size in bytes
/// - `Preview`: the formatted preview string
/// - `HeadroomBudget`: optional token budget for truncation
/// - `meta`: optional metadata key-value pairs
/// - `Center`: optional center annotation
///
/// The rendered block consults the process-global `[templates.marker]`
/// registration ([`super::Const::set_marker_templates`]): when a `Format`
/// string is configured it is rendered with `{preview} {type} {meta} {center}
/// {center_seg} {hash} {size} {hint}` substitution; otherwise the fixed
/// `<<<CCR:{hash}|{type}|{size}>>>` grammar is the fallback.
pub fn CCRMarker(
	HashValue:&str,
	CCRType:&str,
	Size:usize,
	Preview:&str,
	HeadroomBudget:Option<u32>,
	meta:Option<&HashMap<String, String>>,
	Center:Option<&str>,
) -> String {
	// Sanitize preview: newlines → spaces (| is safe - the marker is on its own line).
	let mut Safe = Preview.replace(['\n', '\r'], " ").trim().to_string();
	// Strip control chars
	Safe = Safe.chars().filter(|c| *c >= ' ').collect();

	// Headroom budget truncation
	if let Some(Budget) = HeadroomBudget {
		Safe = match Budget {
			0..=24 => Safe.chars().take(30).collect(),
			25..=49 => Safe.chars().take(60).collect(),
			50..=74 => Safe.chars().take(100).collect(),
			_ => Safe,
		};
	}

	// Metadata string
	let MetaStr = match meta {
		Some(m) => {
			let Parts:Vec<String> = m
				.iter()
				.filter_map(|(k, v)| {
					let Sv = v.replace('|', "/").replace('\n', " ").trim().to_string();
					match Sv.is_empty() {
						true => None,
						false => Some(format!("{}={}", k, Sv)),
					}
				})
				.collect();
			let mut s = Parts.join(";");
			if s.len() > 300 {
				s = format!("{}...", crate::StructExtract::Format::Floor::Fn(&s, 297));
			}
			s
		},
		None => String::new(),
	};

	RenderMarkerBlock(&Safe, CCRType, &MetaStr, Center, HashValue, Size)
}
