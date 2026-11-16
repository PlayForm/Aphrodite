//! The segment marker that separates chained commands in tool output.
//!
//! One constant is the whole wire format between `Marked::Command` (which writes
//! it) and `Marked::Output` (which reads it), so its exact bytes are a contract:
//! change it and already-marked output in flight no longer splits.

/// Marker echoed between segments.
///
/// Wire contract: the bytes are matched verbatim against a trimmed output line
/// (`Marked::Output::Line`), so any change here also changes what a shell prints
/// and what `Marked::Command` emits. Chosen to be visually distinct from real
/// output, unlikely in tool output, and greppable for tests.
pub const Marker:&str = "__APHRODITE_SEG__";
