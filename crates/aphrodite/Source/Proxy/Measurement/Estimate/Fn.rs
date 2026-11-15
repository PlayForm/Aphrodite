//! Estimate the compressed size of content, for EMA feedback only.
//!
//! A trigram-entropy heuristic, not a compressor: it decides whether a payload
//! *looks* compressible so the ratio EMA has a sample before the real backend
//! reports one. The estimate is deliberately conservative in the compress-more
//! direction; it is never used as the stored size.
//!
//! W2-7: the 0.97 coefficient is a DELIBERATE conservative direction - it
//! systematically under-estimates compression for highly repetitive content, so
//! the estimator errs toward compressing MORE (a smaller estimated size → higher
//! ratio → more aggressive thresholds), which is the safe side for a gating
//! heuristic.

use std::collections::HashSet;

/// Estimate the effective compressed byte-size of `Content`.
///
/// Counts unique 3-byte trigrams in the first 4096 bytes. Highly repetitive
/// content (few unique trigrams) yields a small estimate → high compression
/// ratio. Near-random content yields an estimate close to the original size →
/// ratio near 1x. Clamped so the ratio never goes below 1.0x - a compressor
/// cannot expand content beyond the original size in the worst case.
pub(crate) fn Fn(Content:&str) -> usize {
	let bytes = Content.as_bytes();
	let Sample = bytes.get(..4096).unwrap_or(bytes);

	Sample.len().checked_sub(3).map_or_else(
		// Too short for trigrams - a minimal size keeps the ratio
		// reasonable without inflating it.
		|| Sample.len().max(40),
		|_| {
			let Trigrams:HashSet<[u8; 3]> = Sample.windows(3).map(|Window| [Window[0], Window[1], Window[2]]).collect();
			let Unique = Trigrams.len().max(1);
			let Total = Sample.len().saturating_sub(2).max(1);
			// Compressibility: the fraction of the content that is NOT
			// unique. 0.0 = perfectly random (no compression), 1.0 = fully
			// repetitive.
			let Compressibility = 1.0 - (Unique as f64 / Total as f64);
			// estimated = content * (1 - compressibility * 0.97) + overhead
			let Overhead:f64 = 40.0;
			let raw = bytes.len() as f64 * (1.0 - Compressibility * 0.97) + Overhead;

			raw.min(bytes.len() as f64).max(40.0) as usize
		},
	)
}
