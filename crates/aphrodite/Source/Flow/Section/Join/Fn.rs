//! Join the surviving sections into the string the model receives.
//!
//! The only function here that touches rendered text, and it exists so the budget
//! check measures exactly the string the model will receive - measuring any other
//! concatenation would let a turn exceed its budget while the check said it fitted.

/// Join the sections with a single newline between each.
pub(crate) fn Fn(Sections:&[String]) -> String { Sections.join("\n") }
