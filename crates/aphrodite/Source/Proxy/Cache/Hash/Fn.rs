//! FNV-1a 64-bit hash - the INDEX function of the response cache's identity.
//!
//! Chosen over a cryptographic hash because the input is a canonical request body
//! being deduplicated within one process, not a security boundary: collisions are
//! possible in principle, and that is exactly why this is only the index. The
//! identity compares the canonical bytes as well (`Cache::Identity`), so a
//! collision costs one hash bucket, never a wrong answer. It also salts the
//! reload tests' temporary filenames.

/// FNV-1a 64-bit hash over bytes. Deterministic across restarts.
pub(crate) fn Fn(bytes:&[u8]) -> u64 {
	const FnvOffset:u64 = 14695981039346656037;
	const FnvPrime:u64 = 1099511628211;

	bytes
		.iter()
		.fold(FnvOffset, |hash, &b| (hash ^ b as u64).wrapping_mul(FnvPrime))
}
