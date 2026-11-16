//! Join the surviving sections into the string the model receives.
//!
//! The only function here that touches rendered text, and it exists so the budget
//! check measures exactly the string the model will receive - measuring any other
//! concatenation would let a turn exceed its budget while the check said it fitted.
//! `Section::Size` computes the same number without building the string, which is
//! what lets the drop loop subtract instead of re-joining per iteration.

use crate::Flow::Section::Struct::Struct as Section;

/// Join the sections with a single newline between each.
pub(crate) fn Fn(Sections:&[Section]) -> String {
	Sections
		.iter()
		.map(|Section| Section.Text.as_str())
		.collect::<Vec<&str>>()
		.join("\n")
}
