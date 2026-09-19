//! markdown document shape detector.

use crate::preview::input::Input;
use crate::preview::line::md_heading::is_md_heading;
use crate::preview::line::md_structure::is_md_structure;

/// ≥2 heading lines + list/table/link/fence/quote structure or body prose.
/// Gated so comment-only shell scripts cannot trigger it: a doc needs
/// structure OR non-heading body lines - a file of bare `# comment` lines has
/// neither.
pub(crate) fn detect(inp: &Input<'_>) -> bool {
	let md_heads = inp.non_empty.iter().filter(|l| is_md_heading(l)).count();
	let md_structure = inp.non_empty.iter().filter(|l| is_md_structure(l)).count();
	let md_body = inp
		.non_empty
		.iter()
		.filter(|l| !is_md_heading(l) && !is_md_structure(l))
		.count();
	md_heads >= 2 && (md_structure >= 1 || md_body >= 1)
}
