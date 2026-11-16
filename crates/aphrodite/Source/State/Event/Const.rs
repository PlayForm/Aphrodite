//! The tool-event ring's bound.

/// Tool-call events the telemetry ring may hold before the oldest is evicted.
///
/// The same value and the same front eviction as the marker catalog's cap, so
/// "oldest at index 0" holds for both rings and a reader does not have to
/// remember two conventions.
pub(crate) const Cap:usize = 200;
