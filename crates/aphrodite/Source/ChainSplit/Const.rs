//! The segment marker that separates chained commands in tool output.
//!
//! One constant is the whole wire format between `BuildMarkedCommand` (which
//! writes it) and `SplitMarkedOutput` (which reads it), so its exact bytes
//! are a contract: change it and already-marked output in flight no longer
//! splits.

/// Marker echoed between segments. Chosen to be visually distinct from real
/// output, unlikely in tool output, and greppable for tests.
pub const SegMarker:&str = "__APHRODITE_SEG__";
