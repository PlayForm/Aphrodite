//! Request and response body plumbing.
//!
//! The body is where a proxy silently breaks: a cap enforced after the read
//! has already buffered the memory it was meant to save, a stream classified as
//! a buffer, a multibyte char split at a byte budget. Each concern gets its own
//! leaf so none of them is edited by accident while touching another.

pub mod Capped;
pub mod Stream;
pub mod TooLarge;
pub mod Truncate;
