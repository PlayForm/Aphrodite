//! The global (session-less) flag path.
//!
//! Applies only to calls that carry no session context - a non-Hermes host, a
//! standalone tool path, a test. It is deliberately NOT a machine-wide switch:
//! a present global file does not turn debug on for sessions that have their own
//! scoped (absent) flag.

use std::path::PathBuf;

use crate::Debug::{Flag, Runtime};

/// Path of the session-less global debug flag.
///
/// It does not check existence: the read that follows answers `Absent` for a
/// missing file and `Off` for one that exists and says so, so a caller never has
/// to stat first.
pub(crate) fn Fn() -> PathBuf { Runtime::Fn::Fn().join(Flag::Const::FLAG_PREFIX) }
