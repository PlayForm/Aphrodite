//! Classify a tag document as HTML, XML or neither.

use crate::Preview::{Detector::XML::Document::Enum::Enum as Document, Input::Struct::Struct as Input};

/// Document openings that make a payload HTML.
///
/// The order is the contract only in the sense that all three answer `Html`; a payload cannot
/// match two of them at the same position.
const HtmlOpenings:[&str; 3] = ["<!DOCTYPE html", "<html", "<HTML"];

/// Classify a tag document.
///
/// HTML is recognised by its doctype or its root element, spelled in either case; XML is any
/// other document that opens with `<`, closes with `>` and contains a closing tag. It is not a
/// validator: no namespaces, no well-formedness, no encoding declaration.
pub(crate) fn Fn(Inp:&Input<'_>) -> Document {
	let Xml = Inp.Trimmed.starts_with('<') && Inp.raw.trim_end().ends_with('>') && Inp.raw.contains("</");

	match HtmlOpenings.into_iter().find(|Opening| Inp.Trimmed.starts_with(Opening)) {
		Some(_) => Document::Html,
		None => {
			match Xml {
				true => Document::Xml,
				false => Document::Neither,
			}
		},
	}
}
