//! `trait` declarations.

use crate::StructExtract::Declaration::{Enum::Enum as Declaration, Reason::Enum::Enum as Reason};

/// Recognise a Rust trait declaration.
///
/// It is not a Rust parser: a trait's supertraits and generic parameters are cut away, so
/// `trait Foo: Bar` reports `Foo`.
pub(crate) fn Fn(Trimmed:&str) -> Declaration<'_> {
	let Lower = Trimmed.to_lowercase();
	let Declares = Lower.starts_with("trait ") || Lower.starts_with("pub trait ");

	match Declares.then(|| {
		let Name = Trimmed
			.trim_start_matches("pub ")
			.trim_start_matches("trait ")
			.split(|Character:char| Character.is_whitespace() || Character == '<' || Character == '{')
			.next()
			.unwrap_or("");

		match (!Name.is_empty()).then_some(Name) {
			Some(Name) => Declaration::Declaration { Kind:"trait", Name, Parameters:None, Return:None },
			None => Declaration::Malformed { Reason:Reason::NoName },
		}
	}) {
		Some(Declaration) => Declaration,
		None => Declaration::NotDeclaration,
	}
}
