//! Live-container store layout.
//!
//! A live container is not a new store: it is one object written as several
//! views into the existing inline store - one entry per chunk, one whole-content
//! entry, and one metadata envelope, all keyed off the content hash. `Object/`
//! owns that object and the single install pass that writes it, `Key/` derives
//! the view keys, `Chunk/` says how content is cut, and `Marker/` renders the
//! instant marker the model reads.

pub mod Chunk;
pub mod Container;
pub mod Key;
pub mod Marker;
pub mod Object;
