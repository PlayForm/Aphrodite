//! Slice a payload to the requested line window.

/// Slice `Content` to the requested line window.
///
/// `limit == 0` means the FULL document (no cap) - this is what preserves
/// the round-trip content-addressing contract: `retrieve(hash)` must return
/// the exact original bytes, which then hash back to the marker's own hash.
/// Any explicit `Limit` (including one above 10_000) is clamped to a
/// 10,000-line server cap (02-F5). Returns `Err(message)` when `Offset` is
/// at or past the end of the document (mirrors the previous inline
/// `BAD_REQUEST` behavior in [`crate::Retrieve::Handle::Fn::Fn`]). Returns
/// `(content, truncated)`: a `[lines a-b/total]` header is prepended to
/// `Content`, and `Truncated` is `true`, whenever the window doesn't cover
/// the whole document.
pub fn Fn(Content:&str, Offset:usize, Limit:usize) -> Result<(String, bool), String> {
	// Clamp explicit limits: 0 = full document (round-trip contract), any
	// nonzero limit capped at 10_000 lines for safety (02-F5).
	let Limit = if Limit == 0 { usize::MAX } else { Limit.min(10_000) };
	let Lines:Vec<&str> = Content.lines().collect();
	let Total = Lines.len();
	// F20: a zero-line (empty) document is a valid stored entry (e.g.
	// `POST /ccr/create` with `content: ""`), not an out-of-range offset -
	// `offset (0) >= total (0)` used to 400 on this, turning a legitimate
	// create("") -> retrieve round-trip into an error.
	if Total == 0 {
		return Ok((String::new(), false));
	}
	if Offset >= Total {
		return Err(format!("[offset {} out of range; document has {} lines]", Offset, Total));
	}
	let Start = Offset.min(Total);
	// `limit: 0` maps to `usize::MAX` (full document) - saturating add so a
	// nonzero `Start` + `usize::MAX` never overflows (it saturates to MAX,
	// then `.min(total)` lands on `Total` = full document).
	let End = Start.saturating_add(Limit).min(Total);
	// 02-F5: signals to the caller that `Content` is a partial window - via
	// `Offset`, an explicit `Limit`, or an explicit `Limit`'s 10_000-line cap
	// (`limit:0` means the FULL document and never truncates), so truncation
	// is detectable without parsing the `[lines a-b/total]` header below.
	let Truncated = Start > 0 || End < Total;

	// F4: `str::lines()` discards the final line terminator and `join`
	// never restores it, so a full-document retrieval of content ending in
	// "\n" (nearly every source file) came back one byte short of the
	// original - breaking the content-addressing contract (the returned
	// body no longer hashes to the marker's own hash). When the window
	// covers the whole document, skip the lossy lines()/join() round-trip
	// entirely and return the original bytes verbatim.
	if !Truncated {
		return Ok((Content.to_string(), false));
	}
	let mut Windowed = Lines[Start..End].join("\n");
	// A partial window that still reaches the document's end should keep a
	// trailing newline if the original had one, for the same reason.
	if End == Total && Content.ends_with('\n') {
		Windowed.push('\n');
	}
	Windowed = format!("[lines {}-{}/{}]\n{}", Start + 1, End, Total, Windowed);
	Ok((Windowed, Truncated))
}
