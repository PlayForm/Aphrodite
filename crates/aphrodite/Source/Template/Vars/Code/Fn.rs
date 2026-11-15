//! `Code::Fn`: fill the structure variables for code content.

use std::collections::HashMap;

/// Fill `Fns`/`Structs`/`Impls`/`Classes`/`Sigs`/`sig1` from the structure
/// map of `Content`.
///
/// Only called for code types - the caller gates on the type, because the
/// extractor over a log would cost a parse and could invent counts. `code_ts`
/// is extracted as `js` on purpose: the pre-Rust engine used one JavaScript
/// pass for both, and splitting them would change the counts for `.ts` content.
pub(crate) fn Fn(Vars:&mut HashMap<String, String>, TypeStr:&str, Content:&str) {
	let LanguageTag = match TypeStr {
		"code_rust" => "rust",
		"code_python" => "python",
		"code_go" => "go",
		"code_js" => "js",
		"code_ts" => "js",
		"code_sh" => "sh",
		_ => "",
	};
	let St = crate::StructExtract::Extract::Fn(Content, LanguageTag);
	let Sigs = St
		.get("fns")
		.map(|v| v.iter().take(2).cloned().collect::<Vec<_>>().join("; "))
		.unwrap_or_default();
	Vars.insert(
		"fns".into(),
		St.get("fns").map(|v| v.len().to_string()).unwrap_or_else(|| "?".into()),
	);
	Vars.insert(
		"structs".into(),
		St.get("structs").map(|v| v.len().to_string()).unwrap_or_else(|| "0".into()),
	);
	Vars.insert(
		"impls".into(),
		St.get("impls").map(|v| v.len().to_string()).unwrap_or_else(|| "0".into()),
	);
	Vars.insert(
		"classes".into(),
		St.get("classes").map(|v| v.len().to_string()).unwrap_or_else(|| "0".into()),
	);
	Vars.insert("sigs".into(), Sigs.clone());
	// Computed `sig1` = first signature (pre-Rust).
	Vars.insert(
		"sig1".into(),
		Sigs.split(';').next().map(|s| s.trim().to_string()).unwrap_or_default(),
	);
}
