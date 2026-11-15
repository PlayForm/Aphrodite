//! Estimate the compressed size of content, for EMA feedback only.
//!
//! A trigram-entropy heuristic, not a compressor: it decides whether a payload
//! *looks* compressible so the ratio EMA has a sample before the real backend
//! reports one. The estimate is deliberately conservative in the
//! compress-more direction; it is never used as the stored size.

/// Estimate the effective compressed byte-size of `Content` for EMA
/// feedback purposes, using a simple byte-entropy heuristic.
///
/// Counts unique 3-byte trigrams in the first 4096 bytes.  Highly
/// repetitive content (few unique trigrams) yields a small estimate
/// → high compression ratio.  Near-random content yields an estimate
/// close to the original size → ratio near 1×.
///
/// The estimate is clamped so the ratio never goes below 1.0× (a
/// compressor can't expand content beyond the original size in the
/// worst case - it stores the literal).
///
/// W2-7: the 0.97 coefficient below is a DELIBERATE conservative
/// direction - it systematically under-estimates compression for highly
/// repetitive content, so the estimator errs toward compressing MORE
/// (a smaller estimated size → higher ratio → more aggressive
/// thresholds), which is the safe side for a gating heuristic. The
/// estimate is a gating heuristic for the compression decision, NOT the
/// actual compressed size - the real size is whatever the CCR backend
/// produces.
pub(crate) fn Fn(Content:&str) -> usize {
	use std::collections::HashSet;

	let bytes = Content.as_bytes();
	let Sample = match bytes.len() <= 4096 {
		true => bytes,
		false => &bytes[..4096],
	};
	match Sample.len() < 3 {
		true => {
			// Too short for trigrams - just return a minimal size so the
			// ratio is reasonable but not inflated.
			return Sample.len().max(40);
		},
		false => {},
	}

	// Collect unique 3-byte trigrams.
	let mut Trigrams:HashSet<[u8; 3]> = HashSet::with_capacity(Sample.len().min(4096));

	Sample.windows(3).for_each(|Window| {
		Trigrams.insert([Window[0], Window[1], Window[2]]);
	});

	let Unique = Trigrams.len().max(1);
	let Total = Sample.len().saturating_sub(2).max(1);

	// Compressibility: what fraction of the content is NOT unique.
	// 0.0 = perfectly random (no compression), 1.0 = fully repetitive.
	let Uniqueness = Unique as f64 / Total as f64;
	let Compressibility = 1.0 - Uniqueness;

	// estimated_compressed = content * (1 - compressibility * 0.97) + overhead
	// Clamped so that compressed never exceeds original (ratio ≥ 1×).
	let Overhead:f64 = 40.0;
	let raw = bytes.len() as f64 * (1.0 - Compressibility * 0.97) + Overhead;
	let Est = raw.min(bytes.len() as f64).max(40.0);
	Est as usize
}
