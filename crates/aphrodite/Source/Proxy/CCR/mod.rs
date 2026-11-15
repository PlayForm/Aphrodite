//! The `/ccr/*` management surface.
//!
//! Create, list, delete and reload are the operations an external client (the
//! Hermes plugin, an operator's curl) uses to drive the store the proxy owns.
//! They share the store but not a transaction, so each verb is its own leaf.

pub mod Create;
pub mod Delete;
pub mod List;
pub mod Notification;
pub mod Reload;
