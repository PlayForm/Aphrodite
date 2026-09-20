//! git-status shape detector (incl. `On branch ` long-form extension,
//! Phase 6).

use crate::preview::{input::Input, line::git_status::git_status_code};

/// porcelain / `M `/`A `/`D `/`R `/`??`/`UU` prefixes. Require a majority of
/// non-empty lines to carry a status code so a diff hunk (`+`/`-`) or prose
/// isn't mistaken for a status listing.
///
/// Phase 6 extension: `On branch ` long-form (`git status` without
/// `--porcelain`) counts as git too - the proxy classifier recognized it and
/// dropping it would regress those dumps to `text`.
pub(crate) fn detect(inp:&Input<'_>) -> bool {
	let on_branch = inp
		.raw
		.lines()
		.any(|l| l.trim_start().starts_with("On branch ") || l.trim_start().starts_with("Your branch "));
	if on_branch {
		return true;
	}
	let status_lines = inp.non_empty.iter().filter(|l| git_status_code(l).is_some()).count();
	status_lines >= 2 && status_lines * 2 >= inp.non_empty.len()
}
