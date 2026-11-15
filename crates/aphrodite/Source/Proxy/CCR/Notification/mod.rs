//! The webhook that announces a new CCR entry.
//!
//! Four stages, because each one can fail differently: `Event` is the wire
//! payload, `Build` decides whether this create announces itself at all,
//! `Schedule` puts the delivery on the task tracker so shutdown waits for it, and
//! `Deliver` performs the POST and reports one of two outcomes.

pub mod Build;
pub mod Deliver;
pub mod Event;
pub mod Outcome;
pub mod Schedule;
