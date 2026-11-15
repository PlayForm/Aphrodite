//! The per-call LLM hooks.
//!
//! `Pre/` builds the context the model sees before a turn; `Post/` archives
//! the turn and advances the counter. They are the only two points where the
//! turn boundary is observable, so anything that must happen exactly once per
//! turn hangs off them.

pub mod Post;
pub mod Pre;
