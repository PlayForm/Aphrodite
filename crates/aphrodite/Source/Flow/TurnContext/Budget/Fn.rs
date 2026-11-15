//! Drop sections from the bottom until the turn fits its byte budget.
//!
//! Measured on the JOINED string, not on a running total: the newline
//! separators are part of what the model receives, and a per-section sum would
//! let a turn with many small sections exceed the budget while the check said
//! it fit. The loop re-joins on every iteration because the length of the
//! joined string is not a linear function of the section count.

use crate::Flow::Section::Label::Fn::JoinSections;

/// Drop trailing sections until the joined context fits `Budget`, never
/// crossing the always-survive floor.
///
/// Returns the surviving sections and the floor actually used, which the debug
/// line reports - a floor of 0 in the log is how an empty turn is told apart
/// from one that legitimately dropped everything it could.
pub(crate) fn DropToBudget(mut Sections:Vec<String>, AlwaysSurvive:usize, Budget:usize) -> (Vec<String>, usize) {
	let AlwaysSurvive = AlwaysSurvive.min(Sections.len());
	loop {
		match JoinSections(&Sections).len() > Budget && Sections.len() > AlwaysSurvive {
			true => {
				Sections.pop();
			},
			false => break,
		}
	}

	(Sections, AlwaysSurvive)
}
