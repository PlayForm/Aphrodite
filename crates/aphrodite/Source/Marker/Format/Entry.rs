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

use std::collections::BTreeMap;

use super::{
	Block::RenderMarkerBlock,
	Budget::{Enum::Enum as Limit, Fn::Fn as Budget},
};

/// Build a CCR output block.
///
/// - `HashValue`: the content hash
/// - `CCRType`: content type string (e.g. "code_rust", "build")
/// - `Size`: original content size in bytes
/// - `Preview`: the formatted preview string
/// - `HeadroomBudget`: the caller's token budget percentage, or `None` for no
///   truncation at all
/// - `meta`: optional metadata key-value pairs
/// - `Center`: optional center annotation
///
/// The rendered block consults the process-global `[templates.marker]`
/// registration (`super::Registration::Set::Fn`): a `Template` grammar renders
/// with `{preview} {type} {meta} {center} {center_seg} {hash} {size} {hint}`
/// substitution, and `Canonical` is the fixed `<<<CCR:{hash}|{type}|{size}>>>`
/// grammar.
///
/// `meta` is a `BTreeMap` rather than a `HashMap` because its rendering is part
/// of the marker's bytes: a hash map's iteration order would make the same
/// entry render differently between runs, and the `[meta:...]` line is read by
/// the model and by the resolver.
pub fn CCRMarker(
	HashValue:&str,
	CCRType:&str,
	Size:usize,
	Preview:&str,
	HeadroomBudget:Option<u32>,
	meta:Option<&BTreeMap<String, String>>,
	Center:Option<&str>,
) -> String {
	// Sanitize preview in place: newlines → spaces (| is safe - the marker is on
	// its own line), then the C0 controls and DEL's neighbours go.
	let mut Safe = Preview.replace(['\n', '\r'], " ").trim().to_string();
	Safe.retain(|c| c >= ' ');

	// Headroom budget truncation, cut at the byte boundary of the last kept
	// character: no second allocation, and a multi-byte character is never
	// split.
	let Truncate = match Budget(HeadroomBudget) {
		Limit::Unlimited => None,
		Limit::At(Characters) => Safe.char_indices().nth(Characters).map(|(Byte, _)| Byte),
	};
	Truncate.into_iter().for_each(|Byte| Safe.truncate(Byte));

	// Metadata string. `BTreeMap` order is key order, so the joined line is the
	// same on every render.
	let MetaStr = match meta {
		Some(m) => {
			let mut Parts:Vec<String> = m
				.iter()
				.filter_map(|(Key, Value)| {
					let Sv = Value.replace('|', "/").replace('\n', " ").trim().to_string();
					match Sv.is_empty() {
						true => None,
						false => Some(format!("{}={}", Key, Sv)),
					}
				})
				.collect();

			Parts.sort();

			let Joined = Parts.join(";");
			match Joined.len() > 300 {
				true => format!("{}...", crate::StructExtract::Format::Floor::Fn(&Joined, 297)),
				false => Joined,
			}
		},
		None => String::new(),
	};

	RenderMarkerBlock(&Safe, CCRType, &MetaStr, Center, HashValue, Size)
}
