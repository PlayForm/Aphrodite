//! Argument keys whose values are volatile noise.
//!
//! Stripped before hashing so the same logical call hashes the same twice: a
//! timeout that differs, a fresh session id, or a new tool-call id says
//! nothing about WHAT the call did, and leaving them in would make every
//! repeat look new.

/// Keys whose values are volatile noise (change run-to-run for the same logical
/// call) - stripped before hashing so the same command twice yields the same
/// signature (P2/T7).
pub(crate) const VolatileKeys:&[&str] = &["timeout", "timestamp", "session_id", "tool_call_id"];
