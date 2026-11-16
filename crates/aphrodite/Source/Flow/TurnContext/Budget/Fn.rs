//! Drop optional sections from the bottom until the turn fits its budget.
//!
//! Measured on what the JOINED string costs, never on a per-section sum alone: the
//! newline separators are part of what the model receives, and a section-only sum
//! would let a turn with many small sections exceed the budget while the check said
//! it fitted. The measurement is `Section::Size`, which is the joined length
//! computed without joining - so the loop subtracts instead of re-allocating the
//! whole string per iteration, and the two agree by construction (asserted below).
//!
//! The drop rule is two facts kept separate - "too long" and "the last section is
//! droppable" - rather than one compound condition, because the second one is
//! protection and it has to stay visible.
//!
//! The pass is a search, not a loop: the surviving list is the longest prefix of
//! the candidates whose joined size fits, searched from the full list downwards
//! over the trailing optional run. Dropping a section only ever shortens the
//! measurement, so the first prefix that fits is the one the drop loop would have
//! reached, and "no prefix fits" IS the overflow outcome rather than a second test
//! on the bytes afterwards.
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
/// A `Required` section stops the drop: the search only ever shortens the list
/// from the end and only over sections that are `Optional`, so the protected text
/// at the top of the list can never be trimmed away by a budget decision.
pub(crate) fn Fn(mut Sections:Vec<Section>, Budget:usize) -> Outcome {
	debug_assert_eq!(
		JoinedBytes(&Sections),
		JoinSections(&Sections).len(),
		"the incremental measurement must equal the joined string"
	);

	// The trailing optional run is every drop the pass is allowed to make:
	// everything below the last `Required` section is protected by position.
	let Drops = Sections
		.iter()
		.rev()
		.take_while(|Section| Section.Protection == Protection::Optional)
		.count();

	// The longest prefix that fits, searched from the full list downwards. The
	// measurement only shrinks as the prefix does, so the first fit is the prefix
	// the drop loop stopped at; `None` means even the fully-dropped list overruns.
	let Fitting = (0..=Drops)
		.map(|Dropped| Sections.len() - Dropped)
		.find(|Keep| JoinedBytes(&Sections[..*Keep]) <= Budget);

	Sections.truncate(Fitting.unwrap_or(Sections.len() - Drops));

	let Bytes = JoinedBytes(&Sections);
	let Floor = Sections
		.iter()
		.filter(|Section| Section.Protection == Protection::Required)
		.count();
	let Surviving = Measured { Sections, Bytes, Floor };

	match Fitting {
		Some(_) => Outcome::Fits(Surviving),
		None => Outcome::ProtectedOverflow { Excess:Bytes - Budget, Surviving, Limit:Budget },
	}
}
