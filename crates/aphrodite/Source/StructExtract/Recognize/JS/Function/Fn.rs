//! `function` declarations.

use crate::StructExtract::Declaration::{Enum::Enum as Declaration, Reason::Enum::Enum as Reason};

/// The prefixes that open a JS function declaration.
const Prefixes:[&str; 4] = ["function ", "export function ", "async function ", "export async function "];

/// Recognise a JS/TS function declaration.
///
/// The parameters are not recorded: the previous scanner rendered `function <name>` only, and a preview
/// that started reporting parameters would grow every line for no additional decision. It is not a JS
/// parser: a function expression assigned on one line and arrowed on the next is missed.
pub(crate) fn Fn(Trimmed:&str) -> Declaration<'_> {
	let Declares = Prefixes.iter().any(|Prefix| Trimmed.starts_with(Prefix));

	match Declares.then(|| {
		let AfterKeyword = Trimmed
			.trim_start_matches("export ")
			.trim_start_matches("async ")
			.trim_start_matches("function ");
		let Name = AfterKeyword.split('(').next().unwrap_or("").trim();

		match (!Name.is_empty()).then_some(Name) {
			Some(Name) => Declaration::Declaration { Kind:"function", Name, Parameters:None, Return:None },
			None => Declaration::Malformed { Reason:Reason::NoName },
		}
	}) {
		Some(Declaration) => Declaration,
		None => Declaration::NotDeclaration,
	}
}
