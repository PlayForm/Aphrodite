//! One interpretation of an inbound request, taken once.
//!
//! The proxy used to ask two different questions about the same bytes: a
//! substring scan picked the HTTP client and a JSON parse picked the cache
//! policy, so one body could be classified two ways (D3). `Semantics` is now
//! the single answer, produced from the parsed body, and every later stage
//! consumes it rather than re-reading the request.

pub mod Body;
pub mod Cacheability;
pub mod Context;
pub mod Delivery;
pub mod Route;
pub mod Semantics;
