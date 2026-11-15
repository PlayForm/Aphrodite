//! `struct` declarations.

use crate::StructExtract::Declaration::{Enum::Enum as Declaration, Reason::Enum::Enum as Reason};

/// Recognise a Rust struct declaration.
///
/// A tuple struct (`struct Name(T)`) is deliberately not recorded: the previous scanner required the
/// line to contain no parenthesis, and that requirement is what keeps a `struct`-looking call in a
/// match arm from being reported. It is not a Rust parser: generics are cut at `<`, so `struct Foo<T>`
/// reports `Foo`.
pub(crate) fn Fn(Trimmed:&str) -> Declaration {
	let Lower = Trimmed.to_lowercase();
	let Declares = (Lower.starts_with("struct ") || Lower.starts_with("pub struct ")) && !Trimmed.contains('(');

	match Declares.then(|| {
		let Name = Trimmed
			.trim_start_matches("pub ")
			.trim_start_matches("struct ")
			.split(|Character:char| Character.is_whitespace() || Character == '<' || Character == '{')
			.next()
			.unwrap_or("");

		match (!Name.is_empty()).then(|| Name.to_string()) {
			Some(Name) => Declaration::Declaration { Kind:"struct", Name, Parameters:None, Return:None },
			None => Declaration::Malformed { Reason:Reason::NoName },
		}
	}) {
		Some(Declaration) => Declaration,
		None => Declaration::NotDeclaration,
	}
}
