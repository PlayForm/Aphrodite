//! `Extract::Commit::Fn`: the git commit hash.

/// First 7-40 hex run at a line start (git log short/full hash), or the
/// hash of a `commit <hex>` line; pre-Rust default `"???????"`.
///
/// The 7-character floor is what keeps this from matching ordinary words: a
/// 4-character hex run is a plausible identifier or a colour, and treating it
/// as a commit would put a false `{hash}` in every such preview. The result is
/// always truncated to 7, because every consumer compares short hashes.
pub(crate) fn Fn(Lines:&[&str]) -> String {
	Lines
		.iter()
		.find_map(|l| {
			let t = l.trim_start();
			let Candidate = t.strip_prefix("commit ").map(str::trim).unwrap_or(t);
			let Hex:String = Candidate.chars().take_while(|c| c.is_ascii_hexdigit()).collect();
			match Hex.len() >= 7 && Hex.len() <= 40 {
				true => Some(Hex.chars().take(7).collect::<String>()),
				false => None,
			}
		})
		.unwrap_or_else(|| "???????".to_string())
}
