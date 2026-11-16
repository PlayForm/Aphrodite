//! `class` declarations.

use crate::StructExtract::Declaration::{Enum::Enum as Declaration, Reason::Enum::Enum as Reason};

/// Recognise a Python class declaration.
///
/// The name is cut at `(` (a base list) and at `:` (the body). It is not a Python parser: decorators and
/// bases are not read, and indentation is ignored, so a nested class is recorded like a top-level one.
pub(crate) fn Fn(Trimmed:&str) -> Declaration<'_> {
	match Trimmed.strip_prefix("class ") {
		Some(Rest) => {
			let Name = Rest.split(['(', ':']).next().unwrap_or("");

			match (!Name.is_empty()).then_some(Name) {
				Some(Name) => Declaration::Present { Kind:"class", Name, Parameters:None, Return:None },
				None => Declaration::Malformed { Reason:Reason::NoName },
			}
		},
		None => Declaration::Absent,
	}
}
