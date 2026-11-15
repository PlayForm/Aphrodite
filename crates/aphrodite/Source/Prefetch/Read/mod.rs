//! The read half of prefetch: decide what may be read, then read it.
//!
//! Three leaves, in the order they run: `Const/` holds the size cap,
//! `Guard/` absolutizes the path, `Refuse/` applies the denylist, and
//! `Entry/` performs the read. None of them touch state, which is what lets
//! the caller do all the I/O before taking the lock.

pub mod Const;
pub mod Entry;
pub mod Guard;
pub mod Refuse;
