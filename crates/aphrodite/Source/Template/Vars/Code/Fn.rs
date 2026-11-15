//! The code-structure half of the variable set.

use super::Struct::Struct;

/// Read the structure map of a code payload.
///
/// The gate is inside this function rather than at the call site, and it is the
/// pre-Rust gate exactly: `code` and `source_code` carry no language, so the
/// extractor AUTO-DETECTS for them (an empty tag means "ask `Detect`"), while the six
/// `code_*` types name their language. Anything else answers the empty record,
/// because running the extractor over a log would cost a parse and could invent `Fns`
/// counts for content that has no functions.
///
/// `code_ts` is extracted as `js` on purpose: the pre-Rust engine used one JavaScript
/// pass for both, and splitting them would change the counts for `.ts` content.
pub(crate) fn Fn(TypeStr:&str, Content:&str) -> Struct {
	let LanguageTag = match TypeStr {
		"code" | "source_code" => "",
		"code_rust" => "rust",
		"code_python" => "python",
		"code_go" => "go",
		"code_js" | "code_ts" => "js",
		"code_sh" => "sh",
		_ => return Struct::default(),
	};
	let Structure = crate::StructExtract::Extract::Fn(Content, LanguageTag);

	Struct {
		Fns:Structure.get("fns").map(|Signatures| Signatures.len()),
		Structs:Structure.get("structs").map_or(0, |Names| Names.len()),
		Impls:Structure.get("impls").map_or(0, |Names| Names.len()),
		Classes:Structure.get("classes").map_or(0, |Names| Names.len()),
		Sigs:Structure
			.get("fns")
			.map(|Signatures| Signatures.iter().take(2).cloned().collect::<Vec<_>>().join("; "))
			.unwrap_or_default(),
	}
}
