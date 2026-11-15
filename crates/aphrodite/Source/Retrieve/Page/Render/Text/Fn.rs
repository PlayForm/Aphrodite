//! Render a slicing outcome as the wire body.

use super::super::super::Outcome;

/// The response body for a slicing outcome.
///
/// The `[lines a-b/total]` annotation is produced HERE and nowhere else: it is a
/// display of the range, not a property of the content, and keeping it out of
/// [`Outcome`] is what lets `Whole` stay a variant rather than a re-built copy of
/// the document. `Whole` therefore renders the caller's own document, byte for
/// byte.
///
/// The range is 1-based and inclusive on both ends, which is the spelling the
/// annotation has always used and what a reader counts lines with.
pub fn Fn(Content:&str, Outcome:&Outcome::Enum::Enum) -> String {
	match Outcome {
		Outcome::Enum::Enum::Whole => Content.to_string(),
		Outcome::Enum::Enum::Window { Text, Start, End, Total } => format!("[lines {Start}-{End}/{Total}]\n{Text}"),
	}
}
