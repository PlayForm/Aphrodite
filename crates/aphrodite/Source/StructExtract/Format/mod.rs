//! Byte-safe truncation and signature rendering, shared by the language leaves.
//!
//! Every string cut here comes from arbitrary UTF-8 source, so a raw slice at a
//! byte offset can panic on a codepoint boundary; every cut goes through `Floor`
//! first. `Signature` and `Parameters` are the two budget decisions - the whole
//! line and the parenthesised list - kept apart because a caller may need one
//! without the other.

pub mod Floor;
pub mod Parameters;
pub mod Signature;
