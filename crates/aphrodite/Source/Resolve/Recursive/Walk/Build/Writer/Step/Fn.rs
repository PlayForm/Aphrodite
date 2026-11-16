//! One occurrence appended to the output.

use crate::Resolve::{
	Halt,
	Nested,
	Parse::Occurrence::Struct::Struct as Occurrence,
	Recursive::Walk::Build::Writer,
	Traversal::Struct::Struct as Traversal,
};

/// Append the gap before this occurrence and the text that replaces it.
///
/// The budget is charged BEFORE the append, so a bounded expansion stops as soon
/// as the next segment cannot fit instead of building the document and measuring
/// it afterwards. An abandoned nested expansion stops the whole walk: inlining a
/// document that still carries a marker is what the budget exists to prevent.
pub(crate) fn Fn(
	Writer:Writer::Struct::Struct,
	(Occurrence, Resolved):(&Occurrence<'_>, &Nested::Enum::Enum),
	Content:&str,
	Traversal:&mut Traversal,
) -> Result<Writer::Struct::Struct, Halt::Enum::Enum> {
	let Gap = &Content[Writer.Cursor..Occurrence.Span.start];
	let Filled = match Resolved {
		Nested::Enum::Enum::Replaced(Text) => Text.as_str(),
		Nested::Enum::Enum::Unresolved => Occurrence.Marker,
		Nested::Enum::Enum::Abandoned => return Err(Halt::Enum::Enum::Budget),
	};

	Traversal
		.Charge(Writer.Out.len() + Gap.len() + Filled.len())
		.ok_or(Halt::Enum::Enum::Budget)?;

	let mut Out = Writer.Out;
	Out.push_str(Gap);
	Out.push_str(Filled);

	Ok(Writer::Struct::Struct { Out, Cursor:Occurrence.Span.end })
}
