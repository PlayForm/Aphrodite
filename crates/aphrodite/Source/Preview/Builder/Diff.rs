//! Diff preview arm.
//!
//! A diff's line counts say how big a change is but not what it touches, and the model
//! needs the second half. This arm names the changed files. It is not a diff parser: it
//! counts `diff --git` blocks and leading `+`/`-` lines, so a diff without those headers (a
//! bare unified diff) names its files from the `---`/`+++` pairs instead.

use crate::Preview::Input::Struct::Struct as Input;
use super::More::Fn::Fn as More;

/// Files the preview names: three.
const FilesShown:usize = 3;

/// Build a preview of a diff.
///
/// Promises added and removed line counts plus the first couple of changed file names, so
/// the model sees WHAT changed rather than only how much. It is not a semantic diff: a
/// moved block counts as one removal and one addition.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let Files = Inp.raw.matches("diff --git").count();
	let Added = Inp
		.raw
		.lines()
		.filter(|Line| Line.starts_with('+') && !Line.starts_with("+++"))
		.count();
	let Removed = Inp
		.raw
		.lines()
		.filter(|Line| Line.starts_with('-') && !Line.starts_with("---"))
		.count();
	let mut Named:Vec<String> = Inp
		.raw
		.lines()
		.filter_map(|Line| Line.strip_prefix("diff --git "))
		.filter_map(|Rest| Rest.split_whitespace().next())
		.map(|Path| Path.strip_prefix("a/").unwrap_or(Path).to_string())
		.take(FilesShown)
		.collect();

	if Named.is_empty() {
		// Unified `diff -u` / `git diff` without --git headers: name files from the
		// `--- a/path` / `+++ b/path` header pairs so the preview is never `0F` with no
		// file context.
		let mut Seen = std::collections::BTreeSet::new();

		Inp.raw
			.lines()
			.filter_map(|Line| Line.strip_prefix("--- ").or_else(|| Line.strip_prefix("+++ ")))
			.map(|Path| Path.split_whitespace().next().unwrap_or(""))
			.map(|Path| Path.strip_prefix("a/").or_else(|| Path.strip_prefix("b/")).unwrap_or(Path))
			.filter(|Path| !Path.is_empty() && Seen.insert(Path.to_string()))
			.take(FilesShown)
			.for_each(|Path| Named.push(Path.to_string()));
	}

	let More = More(Files, Named.len());
	let Tail = match Named.as_slice() {
		[] => String::new(),
		_ => format!(" | {}{More}", Named.join(" ")),
	};

	format!("[diff:{Files}F +{Added}/-{Removed} {}L{Tail}]", Inp.Total)
}
