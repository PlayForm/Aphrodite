//! The turn-context status renderer for active tasks.
//!
//! Off-path output is byte-identical to the pre-port renderer - the live-container
//! line appears only when `state.live_container` is on, so enabling the feature can
//! never silently reshape the lines an agent has already been conditioned on. Only
//! running tasks are listed: a finished task is the nudge's job, not the status
//! block's.

pub mod Detail;
pub mod Fn;
