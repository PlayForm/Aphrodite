//! Upstream request/response plumbing: what the upstream said, how its body is
//! delivered, and the byte-level helpers both interpreters share.

pub mod Body;
pub mod Head;
pub mod Headers;
pub mod Media;
