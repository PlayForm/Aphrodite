//! `AppState`'s threshold arithmetic: base, inline cutoff, code multiplier,
//! per-content-type scaling.
//!
//! These four methods are the only place a compression decision is made from
//! configuration, and they are read on every request. `ThresholdFor` is the
//! entry point; the rest are its terms.
//!
//! The content family is a `ContentType::Family` value, not a string, so the
//! scaler and the metadata renderer cannot disagree about what a detected name
//! means - and a name neither of them branches on is `Other` explicitly rather
//! than a `_` arm nobody can see.

use std::sync::atomic::Ordering;

use crate::{
	Configuration::Proxy::Mode::Enum::Enum as ProxyMode,
	Proxy::{
		ContentType::Family::{Classify::Fn::Fn as ClassifyFamily, Enum::Enum as ContentFamily},
		State::Struct::Struct as AppState,
	},
};

impl AppState {
	pub(crate) fn CompressThreshold(&self) -> usize {
		match self.Mode {
			ProxyMode::Cache => self.CacheThreshold.load(Ordering::Relaxed),
			ProxyMode::Token => self.TokenThreshold.load(Ordering::Relaxed),
		}
	}

	/// Inline-vs-durable storage cutoff (report 07 F2/T15) - was the
	/// `InlineThreshold` const; now live-configurable via
	/// `compression.inline_threshold`.
	pub(crate) fn InlineThreshold(&self) -> usize { self.InlineThreshold.load(Ordering::Relaxed) }

	/// How many times the base threshold for code content (report 07
	/// F2/F10/T11/T15) - was a free fn parsing `APHRODITE_CODE_MULTIPLIER` as
	/// `usize` (silently truncating the documented `3.0` to a parse failure →
	/// default 2); now live-configurable via `compression.code_multiplier` and
	/// re-resolved as `f64` on every hot-reload.
	pub(crate) fn CodeMultiplier(&self) -> f64 { self.CodeMultiplier.load(Ordering::Relaxed) as f64 / 100.0 }

	/// Per-type threshold - code stays in context longer, logs compressed
	/// aggressively.
	pub(crate) fn ThresholdFor(&self, Ct:&'static str) -> usize {
		let Base = self.CompressThreshold();
		let Family = ClassifyFamily(Ct);

		// Auto-tune: adjust thresholds based on historical compression ratios.
		// Very aggressive compression raises thresholds to preserve more
		// content; very conservative lowers them to compress more. Noisy types
		// are exempt - coding sessions need build output and logs visible at
		// the base threshold, so their multiplier is exactly 1.0.
		let Tune = match Family {
			ContentFamily::Linter | ContentFamily::BuildOutput | ContentFamily::Log => 1.0,
			_ => {
				let Ratio = self.RatioEma.load(Ordering::Relaxed) as f64 / 100.0;

				(Ratio > 20.0)
					.then_some(2.0)
					.or_else(|| (Ratio < 3.0 && Ratio > 0.0).then_some(0.5))
					.unwrap_or(1.0)
			},
		};
		let Base = (Base as f64 * Tune) as usize;

		match Family {
			ContentFamily::Error => Base * 8,
			ContentFamily::CodeRust
			| ContentFamily::CodePython
			| ContentFamily::CodeGo
			| ContentFamily::CodeJS
			| ContentFamily::Code => (Base as f64 * self.CodeMultiplier()) as usize,
			ContentFamily::Diff | ContentFamily::Git | ContentFamily::Text => Base * 2,
			ContentFamily::Linter | ContentFamily::BuildOutput | ContentFamily::Log => Base,
			ContentFamily::ToolOutput | ContentFamily::Json | ContentFamily::Other => Base,
		}
	}
}
