//! The cheap "is anything on?" pre-check.
//!
//! Callers that must capture timing BEFORE deciding whether to log need to know
//! whether the log will be emitted, and they need the answer without paying for
//! the log itself. This leaf is that question, and nothing more.

use crate::Debug::Log::Level;

/// True when any debug channel is on for `session` (cheap pre-check for
/// callers that must capture timing before logging).
///
/// It is not a substitute for `effective_level`: a caller that needs to know
/// WHICH level to log at must ask the level, since this collapses 1 and 2 into
/// the same answer.
pub(crate) fn Fn(state:&aphrodite::State::AphroditeState, session:&str) -> bool { Level::Fn::Fn(state, session) != 0 }
