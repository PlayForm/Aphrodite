//! What a recogniser concluded about one line.

use crate::StructExtract::Declaration::Reason::Enum::Enum as Reason;

/// A line's declaration status, with its parsed parts when it has any.
///
/// `Declaration` carries the pieces rather than a rendered string, so the truncation caps are applied
/// once by the renderer instead of inside every recogniser - which is what let four near-identical
/// scanners drift apart on how wide a parameter list may be.
///
/// It is not a syntax tree: `Parameters` and `Return` are the raw text between the delimiters, never
/// parsed, and a declaration split across lines yields only the half that carries the keyword.
pub(crate) enum Enum {
	/// The line is not a declaration of this shape.
	NotDeclaration,
	/// The line declares something of this shape.
	Declaration { Kind:&'static str, Name:String, Parameters:Option<String>, Return:Option<String> },
	/// The line declares something of this shape, and it could not be read.
	Malformed {
		/// Why the line could not be read. Carried for diagnosis; no consumer
		/// renders it yet, so the field is deliberately unread.
		#[allow(dead_code)]
		Reason:Reason,
	},
}
