//! Which storage target a content length calls for.
//!
//! The comparison is `Length < InlineThreshold`, exactly as the call site had it:
//! content at or above the cutoff is durable only.

use crate::Proxy::Storage::Target::Enum::Enum as StorageTarget;

/// Classify a content length.
pub(crate) fn Fn(Length:usize, InlineThreshold:usize) -> StorageTarget {
	if Length < InlineThreshold {
		StorageTarget::Inline
	} else {
		StorageTarget::Durable
	}
}
