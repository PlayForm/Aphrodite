//! Live-container store layout.
//!
//! A live container is not a new store: it is a naming convention over the
//! existing inline store - one entry per chunk plus one metadata envelope,
//! all keyed off the content hash. These three constants are the whole
//! convention, which is why they live together instead of one per file.

pub mod Chunk;
pub mod Const;
pub mod Container;
pub mod Key;
pub mod Marker;
