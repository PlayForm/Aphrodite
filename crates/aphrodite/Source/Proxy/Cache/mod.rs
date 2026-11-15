//! The response cache: key derivation, the TTL read, and the cache-mode
//! marker.
//!
//! The key must be stable across processes and scoped to one credential; the
//! read must honour the TTL; the marker must be the same grammar the token mode
//! emits. Three independent ways to serve a wrong answer, so three leaves.

pub mod Get;
pub mod Hash;
pub mod Key;
pub mod Marker;
