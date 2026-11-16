//! HTML preview arm.
//!
//! A page's title, heading count and link count say what it is about without shipping the
//! markup. This arm renders those. It is not an HTML parser and it does not execute
//! anything - the tags are read as text, so a `<title>` inside a script string would be
//! found too.

use crate::Preview::Input::Struct::Struct as Input;

/// Characters of the page title the preview shows: sixty.
const TitleShown:usize = 60;

/// Build a preview of HTML content.
///
/// Promises the `<title>` text, a heading count, a link count and a body-size estimate. It
/// is not a DOM: nothing is normalised, so malformed markup yields whatever counts the text
/// happens to produce.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	// Extract `<title>...</title>` text (anywhere on a line, case-insensitive).
	let Title = Inp.raw.lines().find_map(|Line| {
		let Lower = Line.to_lowercase();
		let Start = Lower.find("<title>")? + 7;
		let End = Lower[Start..].find("</title>")?;

		// `get` rather than indexing: the offsets came from the lowercased copy, and a
		// lowercase mapping that changes byte length would otherwise panic here.
		Line.get(Start..Start + End)
			.map(|Title| Title.trim().chars().take(TitleShown).collect::<String>())
	});
	// Count common structural elements.
	let Headings = Inp.raw.matches("<h1").count()
		+ Inp.raw.matches("<h2").count()
		+ Inp.raw.matches("<h3").count()
		+ Inp.raw.matches("<H1").count()
		+ Inp.raw.matches("<H2").count()
		+ Inp.raw.matches("<H3").count();
	let Links = Inp.raw.matches("<a ").count() + Inp.raw.matches("<A ").count();
	let Images = Inp.raw.matches("<img ").count() + Inp.raw.matches("<IMG ").count();
	let Scripts = Inp.raw.matches("<script").count() + Inp.raw.matches("<SCRIPT").count();

	// The tally is the table: a count and its label, rendered only when the count is non-zero.
	let Stats = [(Headings, "h"), (Links, "a"), (Images, "img"), (Scripts, "script")]
		.into_iter()
		.filter(|(Count, _)| *Count > 0)
		.map(|(Count, Label)| format!("{Count}{Label}"))
		.chain(std::iter::once(format!("{}L", Inp.Total)))
		.collect::<Vec<String>>()
		.join(" ");

	match Title {
		Some(Title) => format!("[html:{Stats} | {Title}]"),
		None => format!("[html:{Stats}]"),
	}
}
