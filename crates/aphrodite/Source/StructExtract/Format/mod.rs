//! Byte-safe truncation and signature rendering, shared by the language leaves.
//!
//! Every string cut here comes from arbitrary UTF-8 source, so a raw slice at a
//! byte offset can panic on a codepoint boundary; every cut goes through `Floor`
//! first. `Cap` is the line cap, `Parameters` is the list cap and `Signature` is
//! the plain form of a line - the two budget decisions kept apart because a
//! caller may need one without the other, and one place that measures a finished
//! line so the boundary cannot be spelled two ways.

pub mod Cap;
pub mod Floor;
pub mod Parameters;
pub mod Signature;
