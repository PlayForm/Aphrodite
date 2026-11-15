//! grep/ripgrep hit predicate.

use super::SplitPath;

/// True when a line looks like a grep/ripgrep hit.
///
/// `path:line:match` with a numeric line field, and a path that looks file-ish
/// (contains `/` or a `.ext`). The file-ish requirement is what separates this
/// from the search predicate: a bare `key: value` is not a grep hit. It is not
/// a match extractor - the preview prints the first hit verbatim.
pub(crate) fn Fn(Line:&str) -> bool {
	let (path, _Mid) = match SplitPath::Fn(Line) {
		Some(p) => p,
		None => return false,
	};
	// Path should look file-ish: contain a `/` or a `.ext`.
	path.contains('/') || path.contains('.')
}
