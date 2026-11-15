//! Report whether a slicing outcome is a partial window.

use super::super::super::Outcome;

/// `true` when the served body is a window of a larger document.
///
/// The response carries this so a caller can detect truncation without parsing
/// the `[lines a-b/total]` annotation (02-F5). It is a property of the outcome
/// and not a second flag to keep in step: a window is exactly a body that does
/// not cover its document, and `Whole` is exactly a body that does.
pub fn Fn(Outcome:&Outcome::Enum::Enum) -> bool { matches!(Outcome, Outcome::Enum::Enum::Window { .. }) }
