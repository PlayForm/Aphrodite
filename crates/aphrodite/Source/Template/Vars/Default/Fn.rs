//! The flat half of the variable set.

use std::collections::HashMap;

/// Every name the pre-Rust template namespace exposes, defaulted.
///
/// The defaults ARE the contract, not filler: a template naming `files` or `pid` on
/// content that has neither must render `"?"`, which is what the pre-Rust engine
/// did. Replacing a default with `""` changes every rendered preview that uses it,
/// so they are kept verbatim.
///
/// The variables a later pass always overwrites are seeded here as well. That is not
/// redundancy: it makes the set of NAMES complete before any derivation runs, so the
/// renderer never has to ask whether a key exists and a template can never see a
/// missing variable as an empty one.
///
/// `fx` lives here rather than among the computed variables because it is never
/// derived: `fn` is not recoverable from content alone, and the pre-Rust engine left
/// it blank rather than guessing.
pub(crate) fn Fn(TypeStr:&str, Ln:usize, Size:usize) -> HashMap<String, String> {
	let mut Vars:HashMap<String, String> = HashMap::new();

	Vars.insert("type".into(), TypeStr.to_string());
	Vars.insert("ln".into(), Ln.to_string());
	Vars.insert("size".into(), Size.to_string());
	Vars.insert("first".into(), String::new());
	Vars.insert("err".into(), "0".into());
	Vars.insert("warn".into(), "0".into());
	Vars.insert("msg".into(), String::new());
	Vars.insert("exit".into(), "?".into());
	Vars.insert("cmd".into(), String::new());
	Vars.insert("files".into(), "?".into());
	Vars.insert("fn".into(), String::new());
	Vars.insert("plus".into(), "?".into());
	Vars.insert("minus".into(), "?".into());
	Vars.insert("keys".into(), String::new());
	Vars.insert("items".into(), "?".into());
	Vars.insert("commit".into(), "???????".into());
	Vars.insert("hash".into(), "???????".into());
	Vars.insert("subject".into(), String::new());
	Vars.insert("pid".into(), "?".into());
	Vars.insert("uptime".into(), String::new());
	// Code-structure defaults (pre-Rust): fns "?", structs/impls/classes "0".
	Vars.insert("fns".into(), "?".into());
	Vars.insert("structs".into(), "0".into());
	Vars.insert("impls".into(), "0".into());
	Vars.insert("classes".into(), "0".into());
	Vars.insert("sigs".into(), String::new());
	Vars.insert("code".into(), "?".into());
	Vars.insert("loc".into(), String::new());
	Vars.insert("errors".into(), "0".into());
	Vars.insert("total".into(), Ln.to_string());
	Vars.insert("fx".into(), String::new());

	Vars
}
