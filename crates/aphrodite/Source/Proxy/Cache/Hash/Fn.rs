//! FNV-1a 64-bit hash - the response cache's key function.
//!
//! Chosen over a cryptographic hash because the input is a request body being
//! deduplicated within one process, not a security boundary: collisions are
//! possible in principle, which is why the key also carries the body length and
//! the API key.

/// FNV-1a 64-bit hash over bytes. Deterministic across restarts.
pub(crate) fn Fn(bytes:&[u8]) -> u64 {
	const FnvOffset:u64 = 14695981039346656037;
	const FnvPrime:u64 = 1099511628211;

	bytes
		.iter()
		.fold(FnvOffset, |hash, &b| (hash ^ b as u64).wrapping_mul(FnvPrime))
}
