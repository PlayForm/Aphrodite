//! Typed pre-computed view of a content blob (1.5.0 REFACTOR-PLAN §4).
//!
//! Built ONCE per blob and shared by every detector and every preview arm -
//! no detector or builder ever re-runs `content.lines().collect()`. Predicate
//! scans over `non_empty` are the only per-detector cost.
//!
//! YAGNI note: the plan's reserved fields (`chars`, `first`, `last`,
//! `shebang`, `bracket_balance`, `digit_lines`) landed unconsumed - the
//! builders re-scan `raw.lines()` and the code detector's `#!` check runs
//! through `is_code_strong_line`. Per REFACTOR-PLAN-1.5.0 §9 they are
//! removed rather than kept as dead weight.

/// Pre-computed view of a content blob, shared by every detector and every
/// preview arm. Built once; predicate scans over `non_empty` are the only
/// per-detector cost.
pub(crate) struct Input<'a> {
	pub(crate) raw:&'a str,             // original content
	pub(crate) trimmed:&'a str,         // raw.trim_start()
	pub(crate) non_empty:Vec<&'a str>,  // lines().map(trim_end).filter(|l| !l.trim().is_empty()) - order kept
	pub(crate) total:usize,             // raw.lines().count()   → the `NL` in every [type:NL …] arm
	pub(crate) n:usize,                 // non_empty.len()        → majority denominators
	pub(crate) bytes:usize,             // raw.len()              → the `NB` in error/lint/log/generic arms
}

impl<'a> Input<'a> {
	/// Build the view; `None` when `non_empty` is empty (mirrors the current
	/// early-return in `detect_semantic_type`).
	pub(crate) fn new(content:&'a str) -> Option<Self> {
		let non_empty:Vec<&'a str> = content
			.lines()
			.map(|l| l.trim_end())
			.filter(|l| !l.trim().is_empty())
			.collect();
		if non_empty.is_empty() {
			return None;
		}
		let total = content.lines().count();
		let bytes = content.len();
		let n = non_empty.len();
		Some(Self {
			raw:content,
			trimmed:content.trim_start(),
			non_empty,
			total,
			n,
			bytes,
		})
	}

	/// Count lines matching a predicate.
	pub(crate) fn count(&self, pred:impl Fn(&str) -> bool) -> usize {
		self.non_empty.iter().filter(|l| pred(l)).count()
	}

	/// `count ≥ min && count*2 ≥ n` - the majority rule used by git/grep/ls.
	pub(crate) fn majority(&self, pred:impl Fn(&str) -> bool, min:usize) -> bool {
		let c = self.count(pred);
		c >= min && c * 2 >= self.n
	}

	/// Zero-content view for builders: `build_preview` never early-returns on
	/// empty input (the original `preview.rs` built `[type:0L 0B]` arms), so
	/// the builder path falls back to this when `new()` yields `None`.
	pub(crate) fn empty(content:&'a str) -> Self {
		Self {
			raw:content,
			trimmed:content.trim_start(),
			non_empty:Vec::new(),
			total:content.lines().count(),
			n:0,
			bytes:content.len(),
		}
	}
}