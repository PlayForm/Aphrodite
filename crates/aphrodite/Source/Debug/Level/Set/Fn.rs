//! `SetLevel`: publish the level every call site reads.

use std::sync::atomic::Ordering;

use crate::Debug::Level::Const::Level;

/// Set the process-global debug level (0/1/2). Called from
/// `ConfigurationLoader::apply_compression` so the env var captured at session
/// start reaches every stateless render point (templates, marker format).
///
/// Clamped at 2: the levels are 0/1/2, and a stray `APHRODITE_DEBUG=3` must
/// not open a third behaviour nobody tested - `>= 2` is the verbose test, so 3
/// would silently mean "verbose" while looking like a distinct mode.
pub fn Fn(level:u8) { Level.store(level.min(2), Ordering::Relaxed); }
