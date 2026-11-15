//! One tool event, observed before any decision is taken about it.
//!
//! `Tool/` and `Terminal/` build the observation from the two kinds of call,
//! `Apply/` is the only leaf that records it, and `Struct.rs` is the value they
//! exchange. Observing and deciding are separate steps on purpose: the decision
//! may change, the observation may not be dropped.

pub mod Apply;
pub mod Outcome;
pub mod Struct;
pub mod Terminal;
pub mod Tool;
