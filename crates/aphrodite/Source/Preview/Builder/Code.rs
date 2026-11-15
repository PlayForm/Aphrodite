//! Code preview arm.
//!
//! Source code compressed to a marker still has to tell the model what the code
//! contains, and a byte count does not. This arm renders the structure map -
//! function, struct, trait, impl and class counts plus the first signature - so
//! the dylib/hook path matches the proxy's preview quality. It is not a syntax
//! tree: the map comes from regex extraction, so an unusual dialect can
//! under-count.

use crate::Preview::Input::Struct::Struct as Input;

/// Build a preview of source code.
///
/// Promises a structure map (function, struct, trait, impl, class and type
/// counts) plus the first signature, instead of a bare substring count. It is
/// not a parse: the counts describe what the extractor recognised, so they are
/// a floor, not a census.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let St = crate::StructExtract::Extract::Fn(Inp.raw, "");
	let mut Parts:Vec<String> = Vec::new();
	for (Key, Label) in [
		("fns", "fns"),
		("structs", "structs"),
		("traits", "traits"),
		("impls", "impls"),
		("classes", "classes"),
		("types", "types"),
	] {
		if let Some(v) = St.get(Key)
			&& !v.is_empty()
		{
			Parts.push(format!("{}{}", v.len(), Label));
		}
	}
	let Summary = if Parts.is_empty() {
		format!("{}fns", Inp.raw.matches("fn ").count() + Inp.raw.matches("def ").count())
	} else {
		Parts.join("|")
	};
	let Sig = St
		.get("fns")
		.and_then(|v| v.first())
		.map(|s| format!(" {}", s.chars().take(48).collect::<String>().trim()))
		.unwrap_or_default();
	format!("[code:{}{} {}L]", Summary, Sig, Inp.Total)
}
