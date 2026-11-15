//! Typed pre-computed view of a content blob.
//!
//! Every detector and every preview arm reads the same blob, so the line split,
//! the non-empty filter and the counts are computed once here and shared. That is
//! the entire purpose: no detector re-runs `content.lines().collect()`, and the
//! only per-detector cost is a scan of the already-built line list. It is not a
//! parser - it holds no interpretation of the content, only its shape.

/// Pre-computed view of a content blob.
///
/// Built once and passed to every detector and arm. Fields are plain data: the
/// original text, its left-trimmed form, the non-empty lines with order kept, and
/// the three counts the preview strings quote (`NL`, majority denominators, `NB`).
/// It is not a lifetime-erased copy - every field borrows the caller's content, so
/// the view can never outlive or diverge from it.
pub(crate) struct Struct<'a> {
	pub(crate) raw:&'a str,           // original content
	pub(crate) Trimmed:&'a str,       // raw.trim_start()
	pub(crate) NonEmpty:Vec<&'a str>, // lines().map(trim_end).filter(|l| !l.trim().is_empty()) - order kept
	pub(crate) Total:usize,           // raw.lines().count()   → the `NL` in every [type:NL ...] arm
	pub(crate) n:usize,               // non_empty.len()        → majority denominators
	pub(crate) bytes:usize,           // raw.len()              → the `NB` in error/lint/log/generic arms
}

impl<'a> Struct<'a> {
	/// Build the view, or `None` when the content has no non-empty line.
	///
	/// The `None` is the detector path's early return: there is nothing to
	/// classify. It is not an error - builders use `Empty` instead, because a
	/// preview of empty content is still a preview (`[type:0L 0B]`), while a
	/// classification of empty content is meaningless.
	pub(crate) fn new(Content:&'a str) -> Option<Self> {
		let NonEmpty:Vec<&'a str> = Content.lines().map(|l| l.trim_end()).filter(|l| !l.trim().is_empty()).collect();
		if NonEmpty.is_empty() {
			return None;
		}
		let Total = Content.lines().count();
		let bytes = Content.len();
		let n = NonEmpty.len();
		Some(Self { raw:Content, Trimmed:Content.trim_start(), NonEmpty, Total, n, bytes })
	}

	/// Count non-empty lines matching a predicate.
	///
	/// It is not a count of occurrences: one line matching twice counts once,
	/// which is exactly the property the tallies depend on.
	pub(crate) fn Count(&self, Pred:impl Fn(&str) -> bool) -> usize { self.NonEmpty.iter().filter(|l| Pred(l)).count() }

	/// The majority rule used by git, grep and ls: `count ≥ min` and the count is
	/// at least half the non-empty lines.
	///
	/// It is not a threshold on the total line count: blank lines and trailing
	/// whitespace are already excluded, so a listing with padding still votes.
	pub(crate) fn Majority(&self, Pred:impl Fn(&str) -> bool, Minimum:usize) -> bool {
		let c = self.Count(Pred);
		c >= Minimum && c * 2 >= self.n
	}

	/// Zero-content view for builders.
	///
	/// `BuildPreview` never early-returns on empty input - the original arms
	/// rendered `[type:0L 0B]` - so the builder path falls back to this when `new`
	/// yields `None`. It is not a valid detector input: `n` is zero and every
	/// majority vote fails, which is the correct answer for no content.
	pub(crate) fn Empty(Content:&'a str) -> Self {
		Self {
			raw:Content,
			Trimmed:Content.trim_start(),
			NonEmpty:Vec::new(),
			Total:Content.lines().count(),
			n:0,
			bytes:Content.len(),
		}
	}
}
