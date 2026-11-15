//! The XML detector's answer, in the chain's vocabulary.

use crate::Preview::{
	Detector::XML::{Classify::Fn::Fn as Classify, Document::Enum::Enum as Document},
	Input::Struct::Struct as Input,
};

/// Classify a tag document as `html`, `xml`, or nothing.
///
/// The two type strings are the chain's own vocabulary, so the mapping from the document value
/// happens here and nowhere else. It is not a second classifier: every decision is made by
/// `Classify`.
pub(crate) fn Fn(Inp:&Input<'_>) -> Option<&'static str> {
	match Classify(Inp) {
		Document::Html => Some("html"),
		Document::Xml => Some("xml"),
		Document::Neither => None,
	}
}
