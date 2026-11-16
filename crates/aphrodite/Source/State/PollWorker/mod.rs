//! The poll-worker auto-backgrounding domain's share of the session state.
//!
//! - `Struct.rs` - the background-task deque and the master switch.
//! - `Default.rs` - the seed (no tasks, auto-backgrounding on).

pub mod Default;
pub mod Struct;
