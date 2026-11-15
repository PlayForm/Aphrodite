//! Storing content, and the evidence that it landed.
//!
//! The store used to answer with a `bool`, and every caller that ignored it -
//! or inverted it - could ship a marker whose hash resolves to nothing, which
//! is permanent loss of content the client never received. A marker may now
//! only be built from `Evidence`, which carries the hash and the content whose
//! storage was checked and can be produced only here.
//!
//! What evidence does NOT promise: the entry can be evicted, TTL-expired or
//! removed a moment later. It says the write was accepted and the content is
//! readable now, not that it will be readable when the model retrieves it.

pub mod Evidence;
pub mod Put;
pub mod Store;
pub mod Target;
