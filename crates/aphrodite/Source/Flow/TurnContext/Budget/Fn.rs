//! Drop optional sections from the bottom until the turn fits its budget.
//!
//! Measured on what the JOINED string costs, never on a per-section sum alone: the
//! newline separators are part of what the model receives, and a section-only sum
//! would let a turn with many small sections exceed the budget while the check said
//! it fitted. The measurement is `Section::Size`, which is the joined length
//! computed without joining - so the loop subtracts instead of re-allocating the
//! whole string per iteration, and the two agree by construction (asserted below).
//!
//! `loop` with an explicit predicate rather than a compound `while`: the drop is
//! "repeat while too long AND the last section is droppable", and naming the two
//! facts separately is what keeps the second one - protection - visible instead of
//! buried in a condition.
//!
//! The result is an `Outcome`, not a bare vector: the protected sections are never
//! dropped, so a turn whose required text alone exceeds the budget CANNOT fit, and
//! that has to be a value the caller handles rather than a silent overrun.

use crate::Flow::{
	Section::{
		Join::Fn::Fn as JoinSections,
		Protection::Enum::Enum as Protection,
		Size::Fn::Fn as JoinedBytes,
		Struct::Struct as Section,
	},
	TurnContext::Budget::{Measured::Struct::Struct as Measured, Outcome::Enum::Enum as Outcome},
};

/// Drop trailing optional sections until the joined context fits `Budget`.
///
/// A `Required` section stops the drop: the loop only ever removes from the end
/// and only while the last section is `Optional`, so the protected text at the top
/// of the list can never be trimmed away by a budget decision.
pub(crate) fn DropToBudget(mut Sections:Vec<Section>, Budget:usize) -> Outcome {
	debug_assert_eq!(
		JoinedBytes(&Sections),
		JoinSections(&Sections).len(),
		"the incremental measurement must equal the joined string"
	);

	let mut Bytes = JoinedBytes(&Sections);

	loop {
		let Droppable = Sections
			.last()
			.is_some_and(|Section| Section.Protection == Protection::Optional);

		if Bytes <= Budget || !Droppable {
			break;
		}

		Sections.pop();
		Bytes = JoinedBytes(&Sections);
	}

	let Floor = Sections
		.iter()
		.filter(|Section| Section.Protection == Protection::Required)
		.count();
	let Surviving = Measured { Sections, Bytes, Floor };

	if Bytes > Budget {
		return Outcome::ProtectedOverflow { Excess:Bytes - Budget, Surviving, Limit:Budget };
	}

	Outcome::Fits(Surviving)
}
