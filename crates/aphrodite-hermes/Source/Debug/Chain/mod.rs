//! The session-parent graph and the walk that flattens it into a chain.
//!
//! A debug toggle is scoped to a session TREE, not a session: Hermes spawns
//! subagents with their own ids but the same root, so the flag has to be found
//! by walking upward. This module owns both halves - the learned edges and the
//! bounded walk over them.

mod Fn;
mod State;

pub(crate) use Fn::Fn;
pub(crate) use State::SESSION_PARENTS;
