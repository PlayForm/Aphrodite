//! Diff preview arm.
//!
//! A diff's line counts say how big a change is but not what it touches, and
//! the model needs the second half. This arm names the changed files. It is not
//! a diff parser: it counts `diff --git` blocks and leading `+`/`-` lines, so a
//! diff without those headers (a bare unified diff) reports zero files.

use crate::Preview::Input::Struct::Struct as Input;

/// Build a preview of a diff.
///
/// Promises added and removed line counts plus the first couple of changed file
/// names, so the model sees WHAT changed rather than only how much. It is not a
/// semantic diff: a moved block counts as one removal and one addition.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	let f = Inp.raw.matches("diff --git").count();
	let a = Inp.raw.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
	let d = Inp.raw.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
	let mut Files:Vec<String> = Inp
		.raw
		.lines()
		.filter_map(|l| l.strip_prefix("diff --git "))
		.filter_map(|Rest| Rest.split_whitespace().next())
		.map(|p| p.strip_prefix("a/").unwrap_or(p).to_string())
		.take(3)
		.collect();
	if Files.is_empty() {
		// Unified `diff -u` / `git diff` without --git headers: name
		// files from `--- a/path` / `+++ b/path` header pairs so the
		// preview is never `0F` with no file context (residual #4).
		let mut Seen = std::collections::BTreeSet::new();
		for l in Inp.raw.lines() {
			let p = l.strip_prefix("--- ").or_else(|| l.strip_prefix("+++ "));
			if let Some(p) = p {
				let p = p.split_whitespace().next().unwrap_or("");
				let p = p.strip_prefix("a/").or_else(|| p.strip_prefix("b/")).unwrap_or(p);
				if !p.is_empty() && Seen.insert(p.to_string()) && Files.len() < 3 {
					Files.push(p.to_string());
				}
			}
		}
	}
	if Files.is_empty() {
		format!("[diff:{}F +{}/-{} {}L]", f, a, d, Inp.Total)
	} else {
		let More = if f > Files.len() {
			format!(" +{} more", f - Files.len())
		} else {
			String::new()
		};
		format!("[diff:{}F +{}/-{} {}L | {}{}]", f, a, d, Inp.Total, Files.join(" "), More)
	}
}
