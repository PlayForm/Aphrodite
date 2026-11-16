//! The public marker entry point.
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
	Block::Fn as Block,
	Budget::{Enum::Enum as Limit, Fn::Fn as Budget},
};

/// Build a CCR output block.
///
/// External contract: this is the one function that produces the marker bytes a
/// model reads and `Marker::Parse` reads back, so every argument maps to a field
/// of that wire format.
///
/// - `Hash`: the content hash, rendered verbatim as the marker's first field
/// - `Type`: content type string (e.g. "code_rust", "build")
/// - `Size`: original content size in bytes
/// - `Preview`: the formatted preview string; empty renders an empty preview line
/// - `Headroom`: the caller's token budget percentage, or `None` for no
///   truncation at all (a budget at or above 75 means the same thing)
/// - `Meta`: optional metadata key-value pairs; `None` or empty emits no
///   `[meta:...]` line
/// - `Center`: optional center annotation; `None` falls back to `Type`
///
/// The rendered block consults the process-global `[templates.marker]`
/// registration (`super::Registration::Set::Fn`): a `Template` grammar renders
/// with `{preview} {type} {meta} {center} {center_seg} {hash} {size} {hint}`
/// substitution, and `Canonical` is the fixed `<<<CCR:{hash}|{type}|{size}>>>`
/// grammar.
///
/// `Meta` is a `BTreeMap` rather than a `HashMap` because its rendering is part
/// of the marker's bytes: a hash map's iteration order would make the same
/// entry render differently between runs, and the `[meta:...]` line is read by
/// the model and by the resolver.
pub fn Fn(
	Hash:&str,
	Type:&str,
	Size:usize,
	Preview:&str,
	Headroom:Option<u32>,
	Meta:Option<&BTreeMap<String, String>>,
	Center:Option<&str>,
) -> String {
	// Sanitize preview in place: newlines → spaces (| is safe - the marker is on
	// its own line), then the C0 controls and DEL's neighbours go.
	let mut Safe = Preview.replace(['\n', '\r'], " ").trim().to_string();
	Safe.retain(|Character| Character >= ' ');

	// Headroom budget truncation, cut at the byte boundary of the last kept
	// character: no second allocation, and a multi-byte character is never
	// split.
	let Truncate = match Budget(Headroom) {
		Limit::Unlimited => None,
		Limit::At(Characters) => Safe.char_indices().nth(Characters).map(|(Byte, _)| Byte),
	};
	Truncate.into_iter().for_each(|Byte| Safe.truncate(Byte));

	// Metadata string. `BTreeMap` order is key order, so the joined line is the
	// same on every render.
	let Line = match Meta {
		Some(Map) => {
			let mut Parts:Vec<String> = Map
				.iter()
				.filter_map(|(Key, Value)| {
					let Text = Value.replace('|', "/").replace('\n', " ").trim().to_string();
					match Text.is_empty() {
						true => None,
						false => Some(format!("{}={}", Key, Text)),
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

	Block(&Safe, Type, &Line, Center, Hash, Size)
}
