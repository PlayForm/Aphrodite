//! The `PostLLMCall` expiry pass: write off tasks nobody is polling.
//!
//! Two separate clocks live here and they are not the same number. A running task
//! goes stale when the agent has not polled it for `Age` turns; a
//! finished task is dropped a lifetime after it started, so the agent has a
//! window to retrieve output the nudge already announced. Collapsing the two into
//! one threshold either kills a live build or forgets a completion the agent was
//! told about.

pub mod Apply;
pub mod Const;
pub mod Decide;
pub mod Decision;
pub mod Fn;
pub mod Report;
