//! Render the `X-Aphrodite-Fill-Pct` header value.
//!
//! Three response paths set this header - a cache hit, a compressed response and
//! a passthrough - and each one carried its own copy of the same three lines,
//! including the `is_finite` guard. A header that can be formatted three
//! different ways is a header that will eventually be formatted three different
//! ways.

use std::sync::atomic::Ordering;

use crate::Proxy::State::Struct::Struct as AppState;

impl AppState {
	/// The fill percentage, one decimal place, `0.0` when the stored value is
	/// not finite.
	pub(crate) fn FillText(&self) -> String {
		let Percent = self.FillPct.load(Ordering::Relaxed) as f64 / 100.0;

		// The finiteness test yields the rendered text or nothing; the absent case
		// is the fixed `0.0` the header has always shown for a non-finite fill.
		match Percent.is_finite().then(|| format!("{Percent:.1}")) {
			Some(Text) => Text,
			None => "0.0".to_string(),
		}
	}
}
