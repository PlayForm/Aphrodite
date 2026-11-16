//! A chain's artifacts, built before the session is touched.
//!
//! `Prepare/` is pure and `Apply/` is the only mutating leaf, so the split
//! decision and the store decision stay separable - and a caller that only
//! wants to know whether content is a chain never has to take the state lock.
//!
//! The batch is the unit of ownership: `Struct.rs` closes its fields to this
//! module and derives its byte totals from its own segments, `Segment/` derives
//! each part's correlated artifacts from one text, `Claims/` names which
//! identity a batch reports to the split ledger, and `Apply/` consumes the
//! batch so it cannot be applied twice.

pub mod Apply;
pub mod Claims;
pub mod Prepare;
pub mod Segment;
pub mod Struct;
