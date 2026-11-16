//! Head+tail sampling for very long single-line payloads.
//!
//! A tool result can be one 10 KB line (a minified JSON blob, a single-line log). A bare head
//! truncation of such a line hides the end, and the end is often the part that says what
//! happened. This module samples both ends. It is not a line splitter and it never invents an
//! ellipsis position: the budget is fixed, so two callers sampling the same line get the same
//! string.

use std::num::NonZeroUsize;

/// Longest line shown as a plain head: one hundred characters. A line of exactly this
/// length is short; one character more is sampled.
const Threshold:usize = 100;

/// Characters kept from each end of a sampled line: twenty-eight.
const Length:usize = 28;

/// Characters a short line is cut to: sixty.
const Cap:usize = 60;

/// Render a content hint line, sampling head+tail when the line is very long.
///
/// Lines up to 100 characters are shown as-is under a 60-character cap. A longer line keeps
/// its first and last 28 characters joined by `...`, so both ends are visible instead of a
/// bare 60-character head - and the 61-to-100-character band is still cut to 60, which is what
/// keeps the hint compact. The result is what the model sees, so a change here changes the
/// contract - the numbers are not tunable cosmetics.
///
/// It is not a `Vec<char>` collector: the count is a counter and each end is a `take`/`skip`
/// over the char iterator, so a 10 MB single line costs no allocation beyond the two 28-char
/// samples. Collecting the line first (which this used to do) made the sampling itself the
/// memory the bound was supposed to protect.
pub(crate) fn Fn(Line:&str) -> String {
	let Total = Line.chars().count();

	// `NonZeroUsize` makes "it is a long line" a value instead of a boolean, and the subtraction
	// states the boundary directly: a line of exactly `Threshold` characters is short, one
	// character more is long. The previous `checked_sub(Threshold + 1)` spelled the same
	// boundary with a `+ 1` that read like an off-by-one.
	match NonZeroUsize::new(Total.saturating_sub(Threshold)) {
		Some(_) => {
			let Head:String = Line.chars().take(Length).collect();
			let Tail:String = Line.chars().skip(Total - Length).collect();

			format!("{Head}...{Tail}")
		},
		None => Line.chars().take(Cap).collect(),
	}
}
