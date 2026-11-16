//! `fn` declarations.

use crate::StructExtract::Declaration::{Enum::Enum as Declaration, Reason::Enum::Enum as Reason};

/// The prefixes that open a Rust function declaration.
///
/// The six forms are the ones the previous scanner accepted, including `pub(crate) async fn`, which a
/// version of it missed entirely. A declaration form this list does not name is invisible to the
/// preview, so the list is the contract.
const Prefixes:[&str; 6] = [
	"fn ",
	"pub fn ",
	"async fn ",
	"pub async fn ",
	"pub(crate) fn ",
	"pub(crate) async fn ",
];

/// Recognise a Rust function declaration and read its name, parameters and return type.
///
/// `contains('(')` is required as well as the prefix, because a `fn`-prefixed line with no parenthesis
/// is not a signature. Such a line is reported as `Malformed` with `NoParameters` instead of vanishing
/// into an empty arm, and a declaration whose name field is empty is `Malformed` with `NoName` instead
/// of being rendered as `fn ?(...)`.
///
/// It is not a Rust parser: generics are not tracked, so `impl<T> Foo<T>` reports `Foo`, and a
/// signature split across lines is only half-read.
pub(crate) fn Fn(Trimmed:&str) -> Declaration<'_> {
	let Lower = Trimmed.to_lowercase();
	let Declares = Prefixes.iter().any(|Prefix| Lower.starts_with(Prefix)) && Trimmed.contains('(');

	match Declares.then(|| {
		let AfterKeyword = Trimmed
			.trim_start_matches("pub(crate) ")
			.trim_start_matches("pub ")
			.trim_start_matches("async ")
			.trim_start_matches("fn ");

		let Paren = match AfterKeyword.find('(') {
			Some(Paren) => Paren,
			None => return Declaration::Malformed { Reason:Reason::NoParameters },
		};

		let Name = AfterKeyword.get(..Paren).unwrap_or("");
		let Rest = AfterKeyword.get(Paren..).unwrap_or("");
		let ParametersEnd = Rest.find(')').unwrap_or(Rest.len());
		let Parameters = Rest.get(1..ParametersEnd).unwrap_or("");
		let Return = Rest
			.get(ParametersEnd..)
			.filter(|Tail| Tail.contains("->"))
			.and_then(|Tail| Tail.split("->").nth(1))
			.and_then(|Tail| Tail.split('{').next())
			.map(str::trim)
			.unwrap_or("");

		match (!Name.is_empty()).then_some(Name) {
			Some(Name) => {
				Declaration::Present {
					Kind:"fn",
					Name,
					Parameters:Some(Parameters),
					Return:(!Return.is_empty()).then_some(Return),
				}
			},
			None => Declaration::Malformed { Reason:Reason::NoName },
		}
	}) {
		Some(Declaration) => Declaration,
		None => Declaration::Absent,
	}
}
