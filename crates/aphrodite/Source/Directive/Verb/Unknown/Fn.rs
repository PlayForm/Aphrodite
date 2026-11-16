//! The refusal for a directive name that is not loaded.

use crate::Directive::Refusal;

/// The error value `swap` and `load` answer for an unknown name.
///
/// One leaf because it is one string: the two verbs refuse for the same reason and a caller must
/// not have to tell two spellings of "unknown directive" apart. The string itself lives in
/// [`Refusal::Fn::Fn`], beside the unknown-action refusal, so the directive surface's whole
/// refusal policy is one table. `add` deliberately does NOT refuse - see its own leaf.
pub(crate) fn Fn(name:&str) -> serde_json::Value { Refusal::Fn::Fn(&Refusal::Enum::Enum::Name, name) }
