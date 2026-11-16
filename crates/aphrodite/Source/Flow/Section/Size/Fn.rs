//! The byte length of the joined sections, without joining them.

use crate::Flow::Section::Struct::Struct as Section;

/// The number of bytes `Join` produces for `Sections`.
///
/// It is the sum of the section byte lengths plus one separator per gap - which
/// is exactly what a single `\n` between each pair produces. Measuring this way
/// lets the budget pass subtract as it drops instead of re-allocating the whole
/// joined string on every iteration, and it stays honest because the budget
/// asserts the equality against `Join(...).len()` in debug builds.
pub(crate) fn Fn(Sections:&[Section]) -> usize {
	let Text:usize = Sections.iter().map(|Section| Section.Text.len()).sum();
	let Separators = Sections.len().saturating_sub(1);

	Text.saturating_add(Separators)
}
