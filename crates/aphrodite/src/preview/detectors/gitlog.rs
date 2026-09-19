//! git-log shape detector.

use crate::preview::input::Input;

/// `commit <hash>` blocks: ≥1 block, and either ≥2 blocks or an `Author:`
/// line (a lone `commit ` line in prose is not a log).
pub(crate) fn detect(inp: &Input<'_>) -> bool {
	let commit_lines = inp
		.non_empty
		.iter()
		.filter(|l| {
			l.strip_prefix("commit ")
				.map(|h| h.trim().len() >= 7 && h.trim().chars().take(7).all(|c| c.is_ascii_hexdigit()))
				.unwrap_or(false)
		})
		.count();
	commit_lines >= 1 && (commit_lines >= 2 || inp.raw.contains("Author:"))
}
