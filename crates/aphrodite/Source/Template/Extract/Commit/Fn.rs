//! The git commit hash.

/// First 7-40 hex run at a line start (a git log short/full hash), or the hash of a
/// `commit <hex>` line; `None` when the content carries none.
///
/// The 7-character floor is what keeps this from matching ordinary words: a
/// 4-character hex run is a plausible identifier or a colour, and treating it as a
/// commit would put a false `{hash}` in every such preview. The result is always
/// truncated to 7, because every consumer compares short hashes.
///
/// `None` rather than `"???????"`: the placeholder is the renderer's, and an
/// extractor answering it made "no commit" indistinguishable from a commit whose
/// hash is literally seven question marks.
pub(crate) fn Fn(Lines:&[&str]) -> Option<String> {
	Lines.iter().find_map(|Line| {
		let Trimmed = Line.trim_start();
		let Candidate = Trimmed.strip_prefix("commit ").map(str::trim).unwrap_or(Trimmed);
		let Hex:String = Candidate.chars().take_while(|Char| Char.is_ascii_hexdigit()).collect();

		(7..=40).contains(&Hex.len()).then(|| Hex.chars().take(7).collect::<String>())
	})
}
