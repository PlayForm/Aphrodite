//! The structure-preview entry point.
//!
//! Owns the one budget a preview gets and the dispatch table from a language tag
//! to its extractor. Both belong here rather than in a language leaf: the budget
//! has to be created once per preview so the languages cannot each claim their own,
//! and the table is the only place that knows which tags are recognised.

use std::collections::HashMap;

use crate::StructExtract::{
	Budget::Const as Budget,
	Detect::Fn as AutoDetect,
	Go::Fn as Go,
	JS::Fn as JS,
	Python::Fn as Python,
	Rust::Fn as Rust,
};

/// Extract code structure from source content, as category → signature lines.
///
/// A non-empty `Language` is trusted as given; an empty one means "ask `Detect`",
/// which is what the preview builder passes for a blob whose type was never
/// recorded. Only categories that produced at least one line appear in the map
/// (`Fns`, `Structs`, `Classes`, `Types`, `Traits`, `Impls`, depending on the
/// language), so an absent key means "none found" rather than "not asked for".
///
/// It is not a parser and it does not verify that the content really is the
/// language it was told: an unrecognised tag yields an empty map rather than an
/// error, because a preview with no structure is a degraded marker, not a failure
/// of the compression that produced it. The budget is created here and shared by
/// every category of the call, so the tail categories of a long file are dropped
/// once it is spent.
pub fn Fn(Content:&str, Language:&str) -> HashMap<String, Vec<String>> {
	let LanguageTag = match Language.is_empty() {
		true => AutoDetect(Content),
		false => Language.to_string(),
	};

	match LanguageTag.is_empty() {
		true => {
			return HashMap::new();
		},
		false => {},
	}

	let mut Result:HashMap<String, Vec<String>> = HashMap::new();
	let mut Remaining = Budget as isize;

	match LanguageTag.as_str() {
		"rust" => Rust(Content, &mut Result, &mut Remaining),
		"python" => Python(Content, &mut Result, &mut Remaining),
		"go" => Go(Content, &mut Result, &mut Remaining),
		"js" | "ts" => JS(Content, &mut Result, &mut Remaining),
		_ => {},
	}

	Result
}
