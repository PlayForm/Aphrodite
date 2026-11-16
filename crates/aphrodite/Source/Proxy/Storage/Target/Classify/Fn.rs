//! Which storage target a content length calls for.
//!
//! The comparison is `Length < InlineThreshold`, exactly as the call site had it:
//! content at or above the cutoff is durable only.

use std::cmp::Ordering;

use crate::Proxy::Storage::Target::Enum::Enum as StorageTarget;

/// Classify a content length.
pub(crate) fn Fn(Length:usize, InlineThreshold:usize) -> StorageTarget {
	// The comparison is the value, not a boolean beside it: only `Less` is
	// inline, and the cutoff itself belongs to the durable side.
	match Length.cmp(&InlineThreshold) {
		Ordering::Less => StorageTarget::Inline,
		Ordering::Equal | Ordering::Greater => StorageTarget::Durable,
	}
}
