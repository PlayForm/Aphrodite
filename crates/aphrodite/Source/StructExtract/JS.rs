//! JS/TS extractor: `function`, arrow and `class` signatures.
//!
//! Records `function name` for declarations, `=> name` for bindings whose value is
//! an arrow function, and `class name`, subtracting each line from the shared preview
//! budget as it goes. It is not a JS parser: it reads one line at a time and never
//! tracks a value across lines, so a function expression assigned on one line and
//! arrowed on the next is missed. What must not drift is the arrow guard - a `=>`
//! that only appears in a trailing comment must not turn a plain constant into a
//! function, because the model would then be told the file exports something it does
//! not.

use std::collections::HashMap;

use super::Format::Signature::Fn as Signature;

/// Append this JS/TS source's `Fns` and `Classes` lines to `Result`, spending `Budget`.
///
/// Each signature subtracts its own length plus one separator from the budget, and
/// the scan stops the moment it is spent, so the `Classes` pass only runs when the
/// `Fns` pass left room. A category is inserted only when it produced a line, and the
/// function returns early once the budget is gone. It is not a formatter: the lines
/// stored here are already cut to the signature cap.
pub(super) fn Fn(Content:&str, Result:&mut HashMap<String, Vec<String>>, Budget:&mut isize) {
	let mut Fns:Vec<String> = Vec::new();
	// `Err(())` is the spent-budget sentinel: it stops the scan exactly where the
	// `break` did, and the result is discarded because the budget, not the walk,
	// is the outcome.
	let _ = Content.lines().try_for_each(|Line| -> Result<(), ()> {
		match *Budget <= 0 {
			true => {
				return Err(());
			},
			false => {},
		}
		let Trimmed = Line.trim();
		// function name(...) or export function name(...) or async function name(...)
		match Trimmed.starts_with("function ")
			|| Trimmed.starts_with("export function ")
			|| Trimmed.starts_with("async function ")
			|| Trimmed.starts_with("export async function ")
		{
			true => {
				let After = Trimmed
					.trim_start_matches("export ")
					.trim_start_matches("async ")
					.trim_start_matches("function ");
				let name = After.split('(').next().unwrap_or("?").trim();
				let s = Signature("function", name);
				let Slen = s.len();
				Fns.push(s);
				*Budget -= Slen as isize + 1;
			},
			false => {},
		}
		// Arrow functions: const name = (...) => { ... }
		//
		// T10 (F8): strip a trailing `//` line comment first, THEN split
		// once on the first `=` and require the `=>` to appear on the
		// right-hand side of it - the old check (`trimmed.contains("=>") &&
		// trimmed.contains('=')`) matched `const x = 5; // map => y`,
		// mis-recording the constant `x` as an arrow function purely because
		// a `=>` happened to appear somewhere later in the line, inside a
		// trailing comment rather than the binding's actual value. A naive
		// fix that only special-cased a `//`-PREFIXED line (as opposed to a
		// trailing `// ...` after real code) would still miss this exact
		// case, since the line as a whole does not start with `//`.
		let CodePart = Trimmed.split("//").next().unwrap_or(Trimmed);
		let Split = match CodePart.trim().is_empty() {
			true => None,
			false => CodePart.split_once('='),
		};
		match Split {
			Some((BeforeEq, AfterEq)) => {
				let BeforeEq = BeforeEq.trim();
				match AfterEq.contains("=>")
					&& (BeforeEq.starts_with("const ") || BeforeEq.starts_with("let ") || BeforeEq.starts_with("var "))
				{
					true => {
						let name = BeforeEq
							.trim_start_matches("const ")
							.trim_start_matches("let ")
							.trim_start_matches("var ")
							.trim();
						match !name.is_empty() && name.chars().all(|c| c.is_alphanumeric() || c == '_') {
							true => {
								let s = Signature("=>", name);
								let Slen = s.len();
								Fns.push(s);
								*Budget -= Slen as isize + 1;
							},
							false => {},
						}
					},
					false => {},
				}
			},
			None => {},
		}
		Ok(())
	});
	match Fns.is_empty() {
		false => {
			Result.insert("fns".into(), Fns);
		},
		true => {},
	}
	match *Budget <= 0 {
		true => {
			return;
		},
		false => {},
	}

	let mut Classes:Vec<String> = Vec::new();
	let _ = Content.lines().try_for_each(|Line| -> Result<(), ()> {
		match *Budget <= 0 {
			true => {
				return Err(());
			},
			false => {},
		}
		let Trimmed = Line.trim();
		match Trimmed.strip_prefix("class ") {
			Some(Rest) => {
				let name = Rest.split(['{', ' ', ':']).next().unwrap_or("?");
				let s = Signature("class", name);
				let Slen = s.len();
				Classes.push(s);
				*Budget -= Slen as isize + 1;
			},
			None => {},
		}
		Ok(())
	});
	match Classes.is_empty() {
		false => {
			Result.insert("classes".into(), Classes);
		},
		true => {},
	}
}
