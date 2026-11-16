//! What a recogniser concluded about one line.

use crate::StructExtract::Declaration::Reason::Enum::Enum as Reason;

/// A line's declaration status, with its parsed parts when it has any.
///
/// `Declaration` carries the pieces rather than a rendered string, so the truncation caps are applied
/// once by the renderer instead of inside every recogniser - which is what let four near-identical
/// scanners drift apart on how wide a parameter list may be.
///
/// The parts BORROW the line they were read from. A recogniser reports what it found; the one
/// allocation the preview needs is the rendered line the renderer builds. Owning three `String`s per
/// matched line only to drop them one call later made every recogniser an allocator, and a file with a
/// thousand declarations paid a thousand short-lived allocations for a preview that keeps at most a
/// few hundred bytes of them.
///
/// It is not a syntax tree: `Parameters` and `Return` are the raw text between the delimiters, never
/// parsed, and a declaration split across lines yields only the half that carries the keyword.
pub(crate) enum Enum<'a> {
	/// The line is not a declaration of this shape.
	NotDeclaration,
	/// The line declares something of this shape.
	Declaration {
		Kind:&'static str,
		Name:&'a str,
		Parameters:Option<&'a str>,
		Return:Option<&'a str>,
	},
	/// The line declares something of this shape, and it could not be read.
	Malformed {
		/// Why the line could not be read. Carried for diagnosis; no consumer
		/// renders it yet, so the field is deliberately unread.
		#[allow(dead_code)]
		Reason:Reason,
	},
}
