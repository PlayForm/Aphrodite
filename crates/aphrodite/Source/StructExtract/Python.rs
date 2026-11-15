//! Python extractor: `def` and `class` signatures.
//!
//! Records `def name(params)` for every function or method and `class name` for every
//! class, subtracting each line from the shared preview budget as it goes. It is not a
//! Python parser: it never looks at indentation, so a nested method is recorded exactly
//! like a module-level function and a `def` inside a string is recorded as real. The
//! case-insensitive prefix test is deliberate - `Def` and `Async Def` are still
//! declarations to this reader - and losing it would silently drop every
//! capitalised-`Def` file from the preview.

use std::collections::HashMap;

use crate::StructExtract::{
	Format::{Floor::Fn as Floor, Parameters::Fn as Parameters, Signature::Fn as Signature},
	MaximumSignatureLength::Const as MaximumSigLength,
};

/// Append this Python source's `Fns` and `Classes` lines to `Result`, spending `Budget`.
///
/// Each signature subtracts its own length plus one separator from the budget, and the
/// scan stops the moment it is spent, so the `Classes` pass only runs when the `Fns`
/// pass left room. A category is inserted only when it produced a line, and the function
/// returns early once the budget is gone. It is not a formatter: the lines stored here
/// are already cut to the signature cap.
pub(super) fn Fn(Content:&str, Result:&mut HashMap<String, Vec<String>>, Budget:&mut isize) {
	let mut Fns:Vec<String> = Vec::new();
	for Line in Content.lines() {
		if *Budget <= 0 {
			break;
		}
		let Trimmed = Line.trim();
		let Lower = Trimmed.to_lowercase();
		let IsDef = (Lower.starts_with("def ") || Lower.starts_with("async def ")) && Trimmed.contains('(');
		if IsDef {
			let After = Trimmed.trim_start_matches("async ").trim_start_matches("def ");
			if let Some(Paren) = After.find('(') {
				let name = &After[..Paren];
				let ParamsEnd = After[Paren..].find(')').unwrap_or(0);
				let Params = if ParamsEnd > 1 { &After[Paren + 1..Paren + ParamsEnd] } else { "" };
				let s = format!("def {}({})", name, Parameters(Params));
				let STrunc = if s.len() > MaximumSigLength {
					Floor(&s, MaximumSigLength - 3).to_string() + "..."
				} else {
					s
				};
				let Slen = STrunc.len();
				Fns.push(STrunc);
				*Budget -= Slen as isize + 1;
			}
		}
	}
	if !Fns.is_empty() {
		Result.insert("fns".into(), Fns);
	}
	if *Budget <= 0 {
		return;
	}

	let mut Classes:Vec<String> = Vec::new();
	for Line in Content.lines() {
		if *Budget <= 0 {
			break;
		}
		let Trimmed = Line.trim();
		if let Some(Rest) = Trimmed.strip_prefix("class ") {
			let name = Rest.split(['(', ':']).next().unwrap_or("?");
			let s = Signature("class", name);
			let Slen = s.len();
			Classes.push(s);
			*Budget -= Slen as isize + 1;
		}
	}
	if !Classes.is_empty() {
		Result.insert("classes".into(), Classes);
	}
}
