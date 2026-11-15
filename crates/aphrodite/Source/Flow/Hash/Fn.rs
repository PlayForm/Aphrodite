//! FNV-1a over a byte slice - the turn telemetry's only hash.
//!
//! Chosen because it is order-sensitive, allocation-free and stable across
//! runs; it is a similarity key, NOT a content address. Nothing here is
//! collision-resistant and nothing needs to be: a collision only merges two
//! tool calls into one signature bucket.

use crate::Flow::Hash::Const::{FnvOffset, FnvPrime};

/// FNV-1a 64-bit hash of `bytes`.
pub(crate) fn Fnv1a(bytes:&[u8]) -> u64 {
	let mut hash = FnvOffset;
	for &b in bytes {
		hash ^= b as u64;
		hash = hash.wrapping_mul(FnvPrime);
	}
	hash
}
