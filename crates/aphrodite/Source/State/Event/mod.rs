//! The per-tool-call telemetry ring.
//!
//! One event per call, capped at `Const::Cap` with front eviction - the
//! same eviction style as the marker catalog, so "oldest at index 0" holds for
//! both rings and readers do not have to remember two conventions.
//!
//! Events store hashes, never raw arguments: the ring feeds similarity and
//! error-loop detection, both of which compare digests, so raw args would be
//! PII in state for no analytic gain.
//!
//! - `Struct.rs` - the ring's field.
//! - `Default.rs` - the seed: empty.
//! - `Entry/` - one recorded call (the item).
//! - `Const.rs` - the ring's cap.

pub mod Const;
pub mod Default;
pub mod Entry;
pub mod Record;
pub mod Struct;
