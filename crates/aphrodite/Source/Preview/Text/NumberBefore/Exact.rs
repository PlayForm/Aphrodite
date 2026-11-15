//! Exact-match integer-preceding-keyword parse.

/// Parse the integer immediately preceding `Keyword` on a line.
///
/// `220 passed` yields 220. Returns 0 when the keyword is absent, and also 0
/// when the keyword is present with no digit run in front of it - the two cases
/// are deliberately indistinguishable because a count of zero and no count at
/// all read the same to the preview that consumes them. This is not a
/// case-insensitive match; `Casefold::Fn` exists for that.
pub(crate) fn Fn(Line:&str, Keyword:&str) -> usize {
	let Index = match Line.find(Keyword) {
		Some(i) => i,
		None => return 0,
	};
	Line[..Index]
		.trim_end()
		.rsplit(|c:char| !c.is_ascii_digit())
		.find(|s| !s.is_empty())
		.and_then(|s| s.parse().ok())
		.unwrap_or(0)
}
