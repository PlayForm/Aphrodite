//! Case-insensitive integer-preceding-keyword parse.

/// Parse the integer immediately preceding a case-insensitive `Keyword`.
///
/// `2 Failed` yields 2, and so does `2 failed` - tool output capitalises count
/// words inconsistently, so a case-sensitive read would report a clean run on
/// a failing one. Returns 0 when absent. `0 failed` yields 0 and never counts
/// as a match, which is what keeps a passing test run from previewing as a
/// failure. This replaces the `(?i)[1-9]\d*\s+failed` regex: it scans for the
/// ASCII keyword ignoring case, then parses the digit run before it.
pub(crate) fn Fn(Line:&str, Keyword:&str) -> usize {
	let bytes = Line.as_bytes();
	let k = Keyword.as_bytes();
	let Hit = bytes.len().checked_sub(k.len()).and_then(|Last| {
		(0..=Last).find(|&i| {
			bytes[i..i + k.len()]
				.iter()
				.zip(k.iter())
				.all(|(b, &Kb)| b.eq_ignore_ascii_case(&Kb))
		})
	});

	match Hit {
		Some(i) => {
			let j = bytes[..i].iter().rposition(|b| !b.is_ascii_digit()).map_or(0, |p| p + 1);

			Line[j..i].parse().unwrap_or(0)
		},
		None => 0,
	}
}
