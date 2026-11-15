//! The CCR management surface: create, list, delete, reload, and the webhook
//! that announces a new entry.
//!
//! `/ccr/create` is split the way its own stage comments named it: decode the
//! body, build the store command, execute it, then render the outcome or the
//! rejection. Each stage is a value in and a value out, so the handler is a
//! sequence of decisions rather than a nest of early returns.

pub mod Create;
pub mod Delete;
pub mod List;
pub mod Notification;
pub mod Reload;
