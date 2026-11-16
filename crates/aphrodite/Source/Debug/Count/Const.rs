//! The counter storage.

use std::sync::atomic::AtomicU64;

use crate::Debug::Count::Enum::Enum as Counter;

/// Counter slots - the length of the storage array.
///
/// Private to this file and checked against the enum below, so the two cannot
/// drift: it is not a second source of truth for anything but the array's size.
const Slots:usize = 10;

/// DEV-ONLY cumulative counters (process-global; the dylib hosts one
/// session per process). Indices must match the counter enum.
///
/// A fixed-size array, not a map: the enum is the complete list, the index is
/// the discriminant, and a lock-free `fetch_add` on a slot is the only
/// operation.
pub(crate) static Counters:[AtomicU64; Slots] = [const { AtomicU64::new(0) }; Slots];

/// The storage is exactly one slot per enum discriminant.
///
/// A compile-time assertion rather than a comment: adding a counter variant without
/// growing `Slots` fails to compile here, instead of indexing past the end of the
/// array at the first `fetch_add` in a release build.
const _:() = assert!(Slots == Counter::Warnings as usize + 1);
