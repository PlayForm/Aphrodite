//! One source line and its lowercase form, as a single value.

/// A trimmed line and the lowercase form of the same line.
///
/// Lowercasing is the expensive part of matching, so it is computed once per line and carried
/// with the line it came from. Before, the two travelled as two separate `&str` arguments to a
/// matcher, so nothing stopped a caller passing a line and a DIFFERENT line's lowercase form -
/// a class of bug the type now makes unrepresentable.
///
/// It is not a case-insensitive view of the line: `Trimmed` is the original bytes and `Lower` is a
/// separate owned string, because the matcher needs the exact text to report.
pub(crate) struct Struct<'a> {
	Trimmed:&'a str,
	Lower:String,
}

impl<'a> Struct<'a> {
	pub(crate) fn new(Line:&'a str) -> Self {
		let Trimmed = Line.trim();

		Self { Trimmed, Lower:Trimmed.to_lowercase() }
	}

	/// The line as written, trimmed.
	pub(crate) fn Trimmed(&self) -> &'a str { self.Trimmed }

	/// The lowercase form of `Trimmed`, computed once.
	pub(crate) fn Lower(&self) -> &str { &self.Lower }
}
