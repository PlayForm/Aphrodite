//! Admitting a new task.
//!
//! The decision to background a command lives in `Should`; the identity, the label
//! and the capacity rule live here and in the registry. This leaf only wires them:
//! build the record, hand it to the registry, and let the registry report what it
//! did with it.

pub mod Fn;
