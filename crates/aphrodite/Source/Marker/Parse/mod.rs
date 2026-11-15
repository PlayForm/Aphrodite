//! Reading markers back - the parse half of the wire format.
//!
//! Four independent questions, one file each, because they are asked at
//! different boundaries: a hash handed to a retrieve entry point is normalized
//! before anything else looks at it, validity is a gate, the preview is a
//! display concern, and extraction scans arbitrary text that may contain zero,
//! one or many markers in three different delimiter families.

pub mod Extract;
pub mod Normalize;
pub mod Preview;
pub mod Valid;
