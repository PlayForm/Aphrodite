//! `type ... struct` declarations.

use crate::StructExtract::Declaration::{Enum::Enum as Declaration, Reason::Enum::Enum as Reason};

/// Recognise a Go struct type declaration.
///
/// Only a `struct` type is recorded; a `type Name int` alias is not, because the previous scanner
/// required the word `struct` on the line and the preview has always reported structs under `types`. It
/// is not a Go parser: a `type` block (`type ( ... )`) is not read, since the name and the keyword are
/// not on the same line.
pub(crate) fn Fn(Trimmed:&str) -> Declaration {
	let Declares = Trimmed.starts_with("type ") && Trimmed.contains("struct");

	match Declares.then(|| {
		let Name = Trimmed
			.get("type ".len()..)
			.unwrap_or("")
			.split("struct")
			.next()
			.unwrap_or("")
			.trim();

		match (!Name.is_empty()).then(|| Name.to_string()) {
			Some(Name) => Declaration::Declaration { Kind:"type", Name, Parameters:None, Return:None },
			None => Declaration::Malformed { Reason:Reason::NoName },
		}
	}) {
		Some(Declaration) => Declaration,
		None => Declaration::NotDeclaration,
	}
}
