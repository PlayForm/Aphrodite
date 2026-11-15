//! `impl` blocks.

use crate::StructExtract::Declaration::{Enum::Enum as Declaration, Reason::Enum::Enum as Reason};

/// Recognise a Rust `impl` block and name the type it implements for.
///
/// Both `impl Foo` and `impl<T> Foo<T>` are accepted, and the generic parameter list is skipped so the
/// reported name is the type. It is not a Rust parser: a trait impl (`impl Display for Foo`) reports
/// the trait's name, because the first identifier after the generics is what the previous scanner took
/// and changing it would move every impl line in the preview.
pub(crate) fn Fn(Trimmed:&str) -> Declaration {
	let Lower = Trimmed.to_lowercase();
	let Declares = Lower.starts_with("impl ") || Lower.starts_with("impl<");

	match Declares.then(|| {
		let Name = Trimmed
			.trim_start_matches("impl")
			.trim_start_matches('<')
			.split(|Character:char| Character.is_whitespace() || Character == '<' || Character == '{')
			.find(|Segment| !Segment.is_empty())
			.unwrap_or("");

		match (!Name.is_empty()).then(|| Name.to_string()) {
			Some(Name) => Declaration::Declaration { Kind:"impl", Name, Parameters:None, Return:None },
			None => Declaration::Malformed { Reason:Reason::NoName },
		}
	}) {
		Some(Declaration) => Declaration,
		None => Declaration::NotDeclaration,
	}
}
