//! Process-wide preview length cap.
//!
//! `PreviewMaximumChars` is a process-wide knob: the binary sets it after loading
//! configuration, the proxy re-sets it on hot reload, and the Hermes bridge applies
//! it at dylib init. It therefore lives in one atomic and not in a per-caller
//! struct, so every path sees the same cap. The cap is not advisory - the entry
//! point enforces it on every arm.

pub mod Get;
#[cfg(test)]
pub mod Guard;
pub mod Maximum;
pub mod Set;

