//! HTML versus XML shape detector - the only detector that returns a type.
//!
//! Both shapes are tag documents, and the dedicated HTML arm extracts a `<title>`
//! that XML does not have. This detector separates them. It is not a document
//! parser: it reads the opening token and the presence of a closing tag.

use crate::Preview::Input::Struct::Struct as Input;

/// Classify a tag document as `html` or `xml`.
///
/// A document opening with `<!DOCTYPE html`, `<html` or `<HTML` is `html`; other
/// tag documents with an opening `<` and a `</` close are `xml`. Returns `None`
/// when the content is neither. It is not a validator: a truncated document that
/// opens and closes still classifies, because the arm only needs to know which
/// preview to render.
pub(crate) fn Fn(Inp:&Input<'_>) -> Option<&'static str> {
	if Inp.Trimmed.starts_with("<!DOCTYPE html") || Inp.Trimmed.starts_with("<html") || Inp.Trimmed.starts_with("<HTML")
	{
		return Some("html");
	}
	if Inp.Trimmed.starts_with('<') && Inp.raw.trim_end().ends_with('>') && Inp.raw.contains("</") {
		return Some("xml");
	}
	None
}
