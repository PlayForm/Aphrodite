//! `AppState`'s threshold arithmetic: base, inline cutoff, code multiplier,
//! per-content-type scaling.
//!
//! Split out of the state type because these four methods are the only place
//! a compression decision is made from configuration, and they are read on
//! every request. `ThresholdFor` is the entry point; the rest are its terms.

use std::sync::atomic::Ordering;

use crate::{Configuration::Proxy::Mode::Enum::Enum as ProxyMode, Proxy::State::Struct::Struct as AppState};

impl AppState {
	pub(crate) fn CompressThreshold(&self) -> usize {
		match self.Mode {
			ProxyMode::Cache => self.CacheCompressThreshold.load(Ordering::Relaxed),
			ProxyMode::Token => self.TokenCompressThreshold.load(Ordering::Relaxed),
		}
	}

	/// Inline-vs-durable storage cutoff (report 07 F2/T15) - was the
	/// `InlineCCRThreshold` const; now live-configurable via
	/// `compression.inline_threshold`.
	pub(crate) fn InlineCCRThreshold(&self) -> usize { self.InlineCCRThreshold.load(Ordering::Relaxed) }

	/// How many times the base threshold for code content (report 07
	/// F2/F10/T11/T15) - was a free fn parsing `APHRODITE_CODE_MULTIPLIER` as
	/// `usize` (silently truncating the documented `3.0` to a parse failure
	/// -> default 2); now live-configurable via `compression.code_multiplier`
	/// and re-resolved as `f64` on every hot-reload.
	pub(crate) fn CodeMultiplier(&self) -> f64 { self.CodeMultiplierX100.load(Ordering::Relaxed) as f64 / 100.0 }

	/// Per-type threshold - code stays in context longer, logs compressed
	/// aggressively.
	pub(crate) fn ThresholdFor(&self, Ct:&str) -> usize {
		let Base = self.CompressThreshold();
		// Noisy types: keep at base threshold - coding sessions need build output
		// visible
		match Ct {
			"linter" | "build_output" | "log" => return Base,
			_ => {},
		}
		// Auto-tune: adjust thresholds based on historical compression ratios
		let Ratio = self.CompressionRatioEma.load(Ordering::Relaxed) as f64 / 100.0;
		let Tune = match () {
			// Very aggressive - raise thresholds to preserve more content
			_ if Ratio > 20.0 => 2.0,
			// Very conservative - lower thresholds to compress more
			_ if Ratio < 3.0 && Ratio > 0.0 => 0.5,
			_ => 1.0,
		};
		let Base = (Base as f64 * Tune) as usize;
		match Ct {
			"error" => Base * 8,
			"code_rust" | "code_python" | "code_go" | "code_js" | "code" => {
				(Base as f64 * self.CodeMultiplier()) as usize
			},
			"diff" | "git" => Base * 2,
			"text" => Base * 2,
			"tool_output" => Base,
			"json" => Base,
			_ => Base,
		}
	}
}
