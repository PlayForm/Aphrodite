//! Integer-preceding-keyword parse (e.g. `220 passed` -> 220).

/// Parse the integer immediately preceding `keyword` on a line (e.g. `220
/// passed` -> 220). Returns 0 when absent.
pub(crate) fn num_before(line: &str, keyword: &str) -> usize {
	let idx = match line.find(keyword) {
		Some(i) => i,
		None => return 0,
	};
	line[..idx]
		.trim_end()
		.rsplit(|c: char| !c.is_ascii_digit())
		.find(|s| !s.is_empty())
		.and_then(|s| s.parse().ok())
		.unwrap_or(0)
}

/// Casefold-aware `num_before`: the integer immediately preceding a
/// case-insensitive `keyword` occurrence (e.g. `2 Failed` -> 2); 0 when
/// absent. Replaces the `(?i)[1-9]\d*\s+failed` regex (Phase 4): scans for
/// the ASCII keyword ignoring case, then parses the digit run before it.
/// `0 failed` (clean runs) yields 0 and never matches.
pub(crate) fn num_before_kw(line: &str, keyword: &str) -> usize {
	let bytes = line.as_bytes();
	let k = keyword.as_bytes();
	let mut i = 0;
	while i + k.len() <= bytes.len() {
		let matches = bytes[i..i + k.len()]
			.iter()
			.zip(k.iter())
			.all(|(b, &kb)| b.eq_ignore_ascii_case(&kb));
		if matches {
			let mut j = i;
			while j > 0 && bytes[j - 1].is_ascii_digit() {
				j -= 1;
			}
			return line[j..i].parse().unwrap_or(0);
		}
		i += 1;
	}
	0
}
