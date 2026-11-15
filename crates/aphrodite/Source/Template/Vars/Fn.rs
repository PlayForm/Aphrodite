//! `BuildVars`: the complete template variable set.

use std::collections::HashMap;

use crate::Template::Vars::{Base::Fn::Fn as Base, Code::Fn::Fn as Code, Computed::Fn::Fn as Computed};

/// Build the template variable set from content, mirroring the pre-Rust
/// `_make_ccr_preview` vars dict (plus its `fx`/`Cmx`/`sig1` computed vars).
/// Every extraction is defensive: a value that cannot be derived from the
/// content degrades to its pre-Rust default (`"?"`/`""`/`"0"`), never panics.
///
/// The code branch is gated on the TYPE, not on the content: running the
/// structure extractor over a log would cost a parse and could invent `Fns`
/// counts for content that has no functions, and the pre-Rust engine only
/// enriched code types. `Code` itself is included with an empty language,
/// exactly as the Python side did.
pub(crate) fn Fn(TypeStr:&str, Content:&str) -> HashMap<String, String> {
	let mut Vars = Base(TypeStr, Content);
	if matches!(
		TypeStr,
		"code" | "source_code" | "code_rust" | "code_python" | "code_go" | "code_js" | "code_ts" | "code_sh"
	) {
		Code(&mut Vars, TypeStr, Content);
	}
	Computed(&mut Vars);
	Vars
}
