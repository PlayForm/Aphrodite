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
/// It does not check existence - callers stat the result, so a missing file and
/// an unreadable one both degrade to "off".
pub(crate) fn Fn() -> PathBuf { Runtime::Fn().join(Flag::FLAG_PREFIX) }
