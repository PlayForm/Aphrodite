//! The `/retrieve` request handler.
//!
//! `Fn/` is the order, `Lookup/` is the two-store read and the store it answered from,
//! `Failure/` is why a retrieve cannot serve and `Reply/` is the one place a failure becomes a
//! response. Splitting them is what lets the handler read as the sequence it is rather than as
//! four copies of the same response envelope.

pub mod Failure;
pub mod Fn;
pub mod Lookup;
pub mod Reply;
pub mod Served;
