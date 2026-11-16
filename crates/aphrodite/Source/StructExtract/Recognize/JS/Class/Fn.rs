//! `class` declarations.

use crate::StructExtract::Declaration::{Enum::Enum as Declaration, Reason::Enum::Enum as Reason};

/// Recognise a JS/TS class declaration.
///
/// The name is cut at `{` (the body), at a space (an `extends` clause) and at `:` (a TypeScript type
/// annotation). It is not a JS parser: decorators and members are not read.
pub(crate) fn Fn(Trimmed:&str) -> Declaration<'_> {
	match Trimmed.strip_prefix("class ") {
		Some(Rest) => {
			let Name = Rest.split(['{', ' ', ':']).next().unwrap_or("");

			match (!Name.is_empty()).then_some(Name) {
				Some(Name) => Declaration::Present { Kind:"class", Name, Parameters:None, Return:None },
				None => Declaration::Malformed { Reason:Reason::NoName },
			}
		},
		None => Declaration::Absent,
	}
}
