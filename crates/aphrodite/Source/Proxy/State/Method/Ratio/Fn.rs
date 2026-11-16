//! `AppState`'s compression-ratio feedback loop.
//!
//! An exponential moving average of observed ratios, and the `FillPct` it
//! derives. Both are read by the auto-tune in `ThresholdFor`, so a stalled or
//! wrongly-scaled EMA silently freezes the thresholds - the x100 fixed-point
//! scale is the thing to keep intact.

use std::sync::atomic::Ordering;

use crate::Proxy::State::Struct::Struct as AppState;

impl AppState {
	/// Fold one observed `OriginalLength`/`CompressedLength` pair into the ratio
	/// EMA, then re-derive the fill percentage. `0` on either side is not a
	/// ratio: no sample is taken and the EMA keeps its previous value.
	pub(crate) fn Update(&self, OriginalLength:usize, CompressedLength:usize) {
		// A zero on either side is not a ratio: no sample is taken, and the EMA
		// keeps its previous value. Matching the pair says that once, instead of
		// testing a boolean and then unwrapping the value it stood for.
		let Ratio = match (OriginalLength, CompressedLength) {
			(0, _) | (_, 0) => return,
			(Original, Compressed) => (Original as f64 / Compressed as f64 * 100.0) as u64,
		};

		// Exponential moving average: new = 0.2 * ratio + 0.8 * old
		let Old = self.RatioEma.load(Ordering::Relaxed);
		let Updated = ((Ratio as f64 * 0.2) + (Old as f64 * 0.8)) as u64;

		self.RatioEma.store(Updated, Ordering::Relaxed);
		// After each compression update, also refresh fill_pct for the headroom
		// feedback loop.
		self.Compute();
	}

	/// Derive fill percentage from compression ratio EMA.
	/// Higher compression ratio → lower fill → more headroom.
	/// fill_pct = 100 - (ratio_ema / 20), clamped to [1..99].
	pub(crate) fn Compute(&self) {
		let Current = self.RatioEma.load(Ordering::Relaxed);

		let Pct = match Current {
			0 => 99u64,
			_ => {
				let raw = 100u64.saturating_sub(Current / 20);

				raw.clamp(1, 99)
			},
		};

		self.FillPct.store(Pct * 100, Ordering::Relaxed); // x100 for precision
	}
}
