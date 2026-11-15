//! `Render`: an operation's outcome as the pointer the caller receives.

use std::os::raw::c_char;

use super::super::{Answer::Enum::Enum as Answer, Failure::Fn::Fn as Failure, Transfer::Fn::Fn as Transfer};

/// Render a right or wrong outcome into the one thing the caller sees.
///
/// Success and failure converge here, which is what makes "the error envelope
/// is identical everywhere" a property of the code rather than a convention
/// every export re-earns: `unwrap_or_else` builds the failure payload only when
/// the operation failed.
pub(crate) fn Fn(Outcome:anyhow::Result<Answer>) -> *mut c_char {
	Transfer(Outcome.unwrap_or_else(|Error| Failure(&Error)))
}
