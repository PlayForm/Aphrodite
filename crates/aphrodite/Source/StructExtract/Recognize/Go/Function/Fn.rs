//! `func` declarations, including methods.

use crate::StructExtract::Declaration::{Enum::Enum as Declaration, Reason::Enum::Enum as Reason};

/// Recognise a Go function or method declaration.
///
/// A method's receiver is skipped before the parameter list is read: `func (s *Server) Start(addr
/// string)` reports `Start(addr string)`, not `Start(s *Server)`. Searching for the first `(` in the
/// whole line - which is what the previous scanner did - finds the RECEIVER's parenthesis, and a method
/// whose receiver is reported as its parameter list tells the model the wrong signature.
///
/// It is not a Go parser: braces are never balanced, so a `func` inside a string literal is recorded
/// like a real declaration.
pub(crate) fn Fn(Trimmed:&str) -> Declaration<'_> {
	let Declares = Trimmed.starts_with("func ") && Trimmed.contains('(');

	match Declares.then(|| {
		let AfterKeyword = Trimmed.get("func ".len()..).unwrap_or("");
		// A receiver is the parenthesised group immediately after `func`; its closing paren is where the
		// method's own name starts.
		let ReceiverEnd = AfterKeyword
			.starts_with('(')
			.then(|| AfterKeyword.find(')').map(|End| End + 1))
			.flatten();

		let Named = match ReceiverEnd {
			Some(End) => {
				AfterKeyword
					.get(End..)
					.unwrap_or("")
					.trim_start()
					.split('(')
					.next()
					.unwrap_or("")
			},
			None => AfterKeyword.split('(').next().unwrap_or(""),
		};
		let SearchFrom = match ReceiverEnd {
			Some(End) => "func ".len() + End,
			None => 0,
		};
		let ParametersStart = SearchFrom + Trimmed.get(SearchFrom..).unwrap_or("").find('(').unwrap_or(0);
		let ParametersEnd = Trimmed.get(ParametersStart..).unwrap_or("").find(')').unwrap_or(0);
		let Parameters = match ParametersEnd.checked_sub(1) {
			Some(_) => Trimmed.get(ParametersStart + 1..ParametersStart + ParametersEnd).unwrap_or(""),
			None => "",
		};

		match (!Named.is_empty()).then_some(Named) {
			Some(Name) => Declaration::Present { Kind:"func", Name, Parameters:Some(Parameters), Return:None },
			None => Declaration::Malformed { Reason:Reason::NoName },
		}
	}) {
		Some(Declaration) => Declaration,
		None => Declaration::Absent,
	}
}
