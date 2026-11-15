//! `Tests: N` summary-line predicate.

/// True when a digit immediately follows `Kw` on the line.
///
/// jest writes `Tests:       12 passed, 12 total`, so the digit sits after the
/// keyword rather than before it - the mirror of the pytest shape. It is not a
/// count parser; the detector only needs the yes or no.
pub(crate) fn Fn(Line:&str, Kw:&str) -> bool {
	match Line.find(Kw) {
		Some(i) => {
			Line[i + Kw.len()..]
				.trim_start()
				.chars()
				.next()
				.map(|c| c.is_ascii_digit())
				.unwrap_or(false)
		},
		None => false,
	}
}
