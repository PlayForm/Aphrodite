//! Markdown table shape detector.
//!
//! A table is two or more pipe-prefixed lines plus a separator row. The separator
//! is what distinguishes a table from prose that happens to contain pipes. It is
//! not a cell parser: alignment and cell content are not read.

use crate::Preview::Input::Struct::Struct as Input;

/// True when the content looks like a markdown table.
///
/// Two or more `|`-prefixed lines with at least one separator row. It is not a
/// CSV detector - a comma-separated grid has no pipes and is handled separately.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let MdTableLines = Inp.NonEmpty.iter().filter(|l| l.trim_start().starts_with('|')).count();
	let HasTableSep = Inp.NonEmpty.iter().any(|l| {
		let Cells = l.trim().trim_matches('|');
		!Cells.is_empty()
			&& Cells.split('|').all(|c| {
				let t = c.trim();
				!t.is_empty() && t.chars().all(|Character| matches!(Character, '-' | ':' | ' '))
			})
	});
	MdTableLines >= 2 && HasTableSep
}
