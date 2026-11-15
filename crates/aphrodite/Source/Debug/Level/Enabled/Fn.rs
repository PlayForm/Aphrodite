//! `Enabled`: is the logger on at all.

use crate::Debug::Level::Get::Fn::Fn as Level;

/// Cheap gate: true only when the debug logger is enabled.
///
/// Level 2 is enabled too - "verbose" is a verbosity, not a separate switch,
/// so a caller guarding an expensive capture wants this and not `level() >= 2`.
#[inline]
pub fn Fn() -> bool { Level() != 0 }
