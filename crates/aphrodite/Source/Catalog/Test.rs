//! The catalog battery: item grouping, savings arithmetic, TOC mode and the
//! markdown table's window and overflow notice.

use crate::{
	Catalog::{Build::Fn::Fn as BuildCatalog, Format::Fn::Fn as FormatCatalogTable},
	State::Struct::AphroditeState,
};

#[test]
fn TestEmptyCatalog() {
	let s = AphroditeState::default();
	let Cat = BuildCatalog(&s, "full");
	assert_eq!(Cat["total_items"], 0);
}

#[test]
fn TestCatalogWithItems() {
	let mut s = AphroditeState::default();
	s.RecordMarker(crate::State::Marker::Struct::MarkerEntry {
		hash:"abc123def456".into(),
		CCRType:"code_rust".into(),
		Size:1024,
		Preview:"[code:3fns 42L]".into(),
		Turn:1,
		Center:None,
		meta:None,
	});
	s.RecordMarker(crate::State::Marker::Struct::MarkerEntry {
		hash:"def789abc012".into(),
		CCRType:"diff".into(),
		Size:2048,
		Preview:"[diff:1F +2/-1 10L]".into(),
		Turn:1,
		Center:None,
		meta:None,
	});

	let Cat = BuildCatalog(&s, "full");
	assert_eq!(Cat["total_items"], 2);
	// T6 (F5): "saved" is size minus what still has to be sent (the
	// preview stands in for the rendered marker length) - NOT the raw
	// sum of original sizes, which is what pre-fix code reported here
	// (a flat 3072, i.e. claiming 100% "savings" that were never
	// realized).
	let Expected = (1024 - "[code:3fns 42L]".len()) + (2048 - "[diff:1F +2/-1 10L]".len());
	assert_eq!(Cat["total_saved"], Expected);
}

#[test]
fn TestTocMode() {
	let mut s = AphroditeState::default();
	s.RecordMarker(crate::State::Marker::Struct::MarkerEntry {
		hash:"abc".into(),
		CCRType:"text".into(),
		Size:100,
		Preview:"[text]".into(),
		Turn:1,
		Center:None,
		meta:None,
	});

	let Cat = BuildCatalog(&s, "toc");
	assert!(Cat["hint"].as_str().unwrap().contains("retrieve"));
}

// ── [templates.prompts] search_hint (pre-Rust port): a configured
// template wins over the compiled-in fallback hint. ──
#[test]
fn TestTocHintUsesConfiguredSearchHintTemplate() {
	let mut s = AphroditeState::default();
	s.Templates = Some(crate::State::Template::Struct::TemplatesConfiguration::default());
	s.Templates
		.as_mut()
		.unwrap()
		.Prompts
		.insert("search_hint".into(), "custom search hint".into());
	s.RecordMarker(crate::State::Marker::Struct::MarkerEntry {
		hash:"abc".into(),
		CCRType:"text".into(),
		Size:100,
		Preview:"[text]".into(),
		Turn:1,
		Center:None,
		meta:None,
	});

	let Cat = BuildCatalog(&s, "toc");
	assert_eq!(Cat["hint"].as_str().unwrap(), "custom search hint");
}

#[test]
fn TestEmptyTable() {
	let s = AphroditeState::default();
	assert_eq!(FormatCatalogTable(&s), "No compressed items yet.");
}

#[test]
fn TestByTypeJSONDeterministic() {
	// Workspace serde_json is built with `preserve_order`, so a HashMap
	// iteration order would leak into `ByType` and identical states
	// would serialize differently across calls. BTreeMap must make them
	// byte-identical and sorted.
	let mut s = AphroditeState::default();
	for (i, (hash, CCRType, Preview)) in [
		("h1", "text", "[text] one"),
		("h2", "code_rust", "[code:1 1L]"),
		("h3", "diff", "[diff:1 +1/-0]"),
		("h4", "text", "[text] two"),
	]
	.iter()
	.enumerate()
	{
		s.RecordMarker(crate::State::Marker::Struct::MarkerEntry {
			hash:hash.to_string(),
			CCRType:CCRType.to_string(),
			Size:100 + i * 10,
			Preview:Preview.to_string(),
			Turn:1,
			Center:None,
			meta:None,
		});
	}

	let First = BuildCatalog(&s, "full");
	let Second = BuildCatalog(&s, "full");
	assert_eq!(First["by_type"].to_string(), Second["by_type"].to_string());
	// BTreeMap emits keys sorted, so the order is deterministic and
	// inspectable.
	let Keys:Vec<&str> = First["by_type"].as_object().unwrap().keys().map(|k| k.as_str()).collect();
	assert_eq!(Keys, vec!["code_rust", "diff", "text"]);
}

#[test]
fn TestTableSortsByTurnAndCapsTwenty() {
	// The last-recorded marker carries the OLDEST turn: reverse
	// insertion order would wrongly place it in the newest window.
	let mut s = AphroditeState::default();
	for i in 0..25 {
		s.RecordMarker(crate::State::Marker::Struct::MarkerEntry {
			hash:format!("h{:02}", i),
			CCRType:"text".into(),
			Size:100,
			Preview:format!("[text] entry {}", i),
			Turn:if i == 24 { 1 } else { i },
			Center:None,
			meta:None,
		});
	}
	let Table = FormatCatalogTable(&s);
	let Rows:Vec<&str> = Table.lines().collect();
	// Newest 20 by turn (23 down to 4): the first data row is h23.
	assert!(Rows[4].starts_with("| h23 |"), "newest turn first, got: {}", Rows[4]);
	// The out-of-order old-turn marker and turn <= 3 stay out of the window.
	assert!(!Table.contains("h24"), "oldest-turn marker must not enter the 20-row window");
	assert!(!Table.contains("h03"), "only the 20 newest turns may be listed");
	// Header claims 25 items, body shows 20 - the notice reconciles it.
	assert!(Table.contains("5 more entries"), "missing overflow notice: {}", Table);
	assert!(Table.contains("aphrodite_catalog"), "notice must point at the catalog tool");
}
