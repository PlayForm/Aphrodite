//! Markdown document shape detector.
//!
//! Headings alone are not a document - a shell script full of `# comment` lines
//! looks identical. This detector therefore requires structure or body prose
//! alongside the headings. It is not a markdown renderer: nothing is formatted.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Line::Markdown::{Heading, Structure},
};

/// True when the content looks like a markdown document.
///
/// Two or more heading lines, plus either list/table/link/fence/quote structure
/// or non-heading body lines. The gate exists so comment-only shell scripts
/// cannot trigger it: a file of bare `# comment` lines has neither. It is not a
/// document classifier - front matter and inline markup are ignored.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let MdHeads = Inp.NonEmpty.iter().filter(|l| Heading::Fn(l)).count();
	let MdStructure = Inp.NonEmpty.iter().filter(|l| Structure::Fn(l)).count();
	let MdBody = Inp.NonEmpty.iter().filter(|l| !Heading::Fn(l) && !Structure::Fn(l)).count();
	MdHeads >= 2 && (MdStructure >= 1 || MdBody >= 1)
}
