//! The in-process record of the current turn's session.
//!
//! Updated only by `record_session` - i.e. the `pre_llm_call` hook - never by a
//! transform hook. That distinction matters: a subagent's tool result arriving
//! mid-turn must not redirect the debug toggle away from the turn's session.

use std::sync::{Mutex, OnceLock};

/// The most recently seen session id, updated ONLY by `record_session` -
/// i.e. the `pre_llm_call` hook (turn_context.py threads session_id through
/// it). The `aphrodite_debug` tool has no session in its args: it toggles
/// whatever session the current LLM turn belongs to, so the flag must track
/// the turn's session, NOT whichever transform hook fired last (subagent
/// results could otherwise redirect the toggle to the wrong root).
pub(crate) static LAST_SESSION:OnceLock<Mutex<String>> = OnceLock::new();
