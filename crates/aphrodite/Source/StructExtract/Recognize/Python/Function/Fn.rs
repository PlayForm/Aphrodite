//! `def` declarations.

use crate::StructExtract::Declaration::{Enum::Enum as Declaration, Reason::Enum::Enum as Reason};

/// Recognise a Python function or method declaration.
///
/// The prefix test is case-insensitive on purpose: `Def` and `Async Def` are still declarations to this
/// reader, and losing that would silently drop every capitalised-`Def` file from the preview. It is not
/// a Python parser: indentation is never looked at, so a nested method is recorded exactly like a
/// module-level function and a `def` inside a string is recorded as real.
pub(crate) fn Fn(Trimmed:&str) -> Declaration<'_> {
	let Lower = Trimmed.to_lowercase();
	let Declares = (Lower.starts_with("def ") || Lower.starts_with("async def ")) && Trimmed.contains('(');

	match Declares.then(|| {
		let AfterKeyword = Trimmed.trim_start_matches("async ").trim_start_matches("def ");

		let Paren = match AfterKeyword.find('(') {
			Some(Paren) => Paren,
			None => return Declaration::Malformed { Reason:Reason::NoParameters },
		};

		let Name = AfterKeyword.get(..Paren).unwrap_or("");
		let ParametersEnd = AfterKeyword.get(Paren..).unwrap_or("").find(')').unwrap_or(0);
		let Parameters = match ParametersEnd > 1 {
			true => AfterKeyword.get(Paren + 1..Paren + ParametersEnd).unwrap_or(""),
			false => "",
		};

		match (!Name.is_empty()).then_some(Name) {
			Some(Name) => Declaration::Present { Kind:"def", Name, Parameters:Some(Parameters), Return:None },
			None => Declaration::Malformed { Reason:Reason::NoName },
		}
	}) {
		Some(Declaration) => Declaration,
		None => Declaration::Absent,
	}
}
