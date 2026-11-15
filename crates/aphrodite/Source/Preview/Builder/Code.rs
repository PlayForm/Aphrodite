//! Code preview arm.
//!
//! Source code compressed to a marker still has to tell the model what the code contains,
//! and a byte count does not. This arm renders the structure map - function, struct,
//! trait, impl and class counts plus the first signature - so the dylib/hook path matches
//! the proxy's preview quality. It is not a syntax tree: the map comes from the structure
//! extractor, so an unusual dialect can under-count.

use crate::Preview::Input::Struct::Struct as Input;

/// Characters of the first signature the preview shows: forty-eight.
const SignatureShown:usize = 48;

/// Build a preview of source code.
///
/// Promises a structure map (function, struct, trait, impl, class and type counts) plus
/// the first signature, instead of a bare substring count. It is not a parse: the counts
/// describe what the extractor recognised, so they are a floor, not a census.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Structure = crate::StructExtract::Extract::Fn(Inp.raw, "");
	let Parts:Vec<String> = ["fns", "structs", "traits", "impls", "classes", "types"]
		.into_iter()
		.filter_map(|Key| {
			Structure
				.get(Key)
				.filter(|Signatures| !Signatures.is_empty())
				.map(|Signatures| format!("{}{Key}", Signatures.len()))
		})
		.collect();
	let Summary = match Parts.as_slice() {
		[] => format!("{}fns", Inp.raw.matches("fn ").count() + Inp.raw.matches("def ").count()),
		_ => Parts.join("|"),
	};
	let Signature = Structure
		.get("fns")
		.and_then(|Signatures| Signatures.first())
		.map(|Signature| format!(" {}", Signature.chars().take(SignatureShown).collect::<String>().trim()))
		.unwrap_or_default();

	format!("[code:{Summary}{Signature} {}L]", Inp.Total)
}
