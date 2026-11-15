//! Policy bounds for the poll worker: how many tasks it tracks at once and
//! how long an unpolled task survives before it is written off.
//!
//! Both numbers are policy, not tuning knobs. `MaximumBgTasks` bounds the
//! per-turn status render and the nudge budget; `StaleTurnAge` is the
//! point at which a task the agent stopped polling is marked abandoned. A
//! task marked stale too early looks like a dead build, so the value is
//! public and shared with the expiry pass rather than duplicated.

/// Maximum concurrent background tasks tracked.
pub(crate) const MaximumBgTasks:usize = 4;

/// Tasks older than this many turns without a poll are considered stale
/// and auto-expired at `PostLLMCall`.
pub const StaleTurnAge:usize = 8;
