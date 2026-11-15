//! Recognise one markdown heading.

use crate::Preview::{Line::MarkdownHeading, Parse::Markdown::Heading::Struct::Struct as Heading};

/// Reduce a heading line to its level and text, or `None` when it is not a heading.
///
/// The level is the run of leading hashes, and the shared heading predicate is what
/// decides whether the line is a heading at all (one to six hashes, a space, and real
/// text). It is not a markdown parser: a heading-looking line inside a fenced block is
/// still counted, which the preview has never claimed to exclude.
pub(crate) fn Fn(Line:&str) -> Option<Heading> {
	let Trimmed = Line.trim_start();
	let Level = Trimmed.chars().take_while(|Character| *Character == '#').count();

	MarkdownHeading::Fn(Line).then(|| Heading { Level, Text:Trimmed.to_string() })
}
