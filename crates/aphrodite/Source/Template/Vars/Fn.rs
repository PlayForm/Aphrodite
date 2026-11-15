//! The complete template variable set.

use std::collections::HashMap;

use super::{
	Code::Fn::Fn as Code,
	Computed::Fn::Fn as Computed,
	Default::Fn::Fn as Defaults,
	Derive::Fn::Fn as Derive,
};

/// Build the template variable set from content, mirroring the pre-Rust
/// `_make_ccr_preview` vars dict (plus its `fx`/`cmx`/`sig1` variables).
///
/// Four passes, in an order that is part of the contract: the flat defaults, then the
/// content-derived values, then the structure map, then the variables derived from
/// other variables. Every extraction is defensive - a value that cannot be derived
/// from the content renders its pre-Rust default (`"?"`/`""`/`"0"`), never panics.
///
/// The three passes that produce values return TYPED RECORDS and this function
/// renders them. That is the point of the split: a reader of `Derive::Fn::Fn` cannot
/// invent a placeholder, and the namespace's defaults are visible in one place rather
/// than spread over four scanners.
pub(crate) fn Fn(TypeStr:&str, Content:&str) -> HashMap<String, String> {
	let Lines:Vec<&str> = Content.lines().collect();
	let Lower:Vec<String> = Lines.iter().map(|Line| Line.to_ascii_lowercase()).collect();

	let mut Vars = Defaults(TypeStr, Lines.len(), Content.len());

	Derive(&mut Vars, Content, &Lines, &Lower);

	// The empty record is the non-code answer, so the "is this code" gate is the
	// record's own content rather than a branch here.
	let Structure = Code(TypeStr, Content);

	Vars.insert(
		"fns".into(),
		Structure.Fns.map_or_else(|| "?".to_string(), |Fns| Fns.to_string()),
	);
	Vars.insert("structs".into(), Structure.Structs.to_string());
	Vars.insert("impls".into(), Structure.Impls.to_string());
	Vars.insert("classes".into(), Structure.Classes.to_string());
	// `sig1` is the first signature of the `sigs` string, so it is derived from the
	// same value rather than computed twice.
	Vars.insert(
		"sig1".into(),
		Structure
			.Sigs
			.split(';')
			.next()
			.map(|Signature| Signature.trim().to_string())
			.unwrap_or_default(),
	);
	Vars.insert("sigs".into(), Structure.Sigs);

	// `cmx` reads `cmd`, so it runs last. `get` rather than indexing: a missing key
	// must read as "no command", not panic.
	let Computed = Computed(Vars.get("cmd").map_or("", String::as_str));
	Vars.insert("cmx".into(), Computed.Cmx);

	Vars
}
