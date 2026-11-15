//! The counter storage.

use std::sync::atomic::AtomicU64;

/// DEV-ONLY cumulative counters (process-global; the dylib hosts one
/// session per process). Indices must match the counter enum.
///
/// A fixed-size array, not a map: the enum is the complete list, the index is
/// the discriminant, and a lock-free `fetch_add` on a slot is the only
/// operation. Growing this array without growing the enum (or the reverse) is
/// the one way to get this wrong - the enum lives in `Enum/` and both are
/// declared in this directory so they move together.
pub(crate) static Counters:[AtomicU64; 10] = [const { AtomicU64::new(0) }; 10];
