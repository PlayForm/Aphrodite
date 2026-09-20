//! html vs xml shape detector - the only non-bool detector.

use crate::preview::input::Input;

/// A document that opens with `<!DOCTYPE html`/`<html` is `html` (the
/// dedicated arm extracts `<title>`); other tag documents with an opening `<`
/// and a `</` close count as `xml`. Returns `Some("html" | "xml")` or `None`.
pub(crate) fn detect(inp:&Input<'_>) -> Option<&'static str> {
	if inp.trimmed.starts_with("<!DOCTYPE html") || inp.trimmed.starts_with("<html") || inp.trimmed.starts_with("<HTML")
	{
		return Some("html");
	}
	if inp.trimmed.starts_with('<') && inp.raw.trim_end().ends_with('>') && inp.raw.contains("</") {
		return Some("xml");
	}
	None
}
