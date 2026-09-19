//! diff preview arm (names the changed files).

use crate::preview::input::Input;

/// Enrich: name the first couple of changed files so the agent sees WHAT
/// changed, not just how many lines.
pub(crate) fn build_diff_preview(inp:&Input<'_>) -> String {
	let f = inp.raw.matches("diff --git").count();
	let a = inp.raw.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
	let d = inp.raw.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
	let mut files:Vec<String> = inp
		.raw
		.lines()
		.filter_map(|l| l.strip_prefix("diff --git "))
		.filter_map(|rest| rest.split_whitespace().next())
		.map(|p| p.strip_prefix("a/").unwrap_or(p).to_string())
		.take(3)
		.collect();
	if files.is_empty() {
		// Unified `diff -u` / `git diff` without --git headers: name
		// files from `--- a/path` / `+++ b/path` header pairs so the
		// preview is never `0F` with no file context (residual #4).
		let mut seen = std::collections::BTreeSet::new();
		for l in inp.raw.lines() {
			let p = l.strip_prefix("--- ").or_else(|| l.strip_prefix("+++ "));
			if let Some(p) = p {
				let p = p.split_whitespace().next().unwrap_or("");
				let p = p.strip_prefix("a/").or_else(|| p.strip_prefix("b/")).unwrap_or(p);
				if !p.is_empty() && seen.insert(p.to_string()) && files.len() < 3 {
					files.push(p.to_string());
				}
			}
		}
	}
	if files.is_empty() {
		format!("[diff:{}F +{}/-{} {}L]", f, a, d, inp.total)
	} else {
		let more = if f > files.len() {
			format!(" +{} more", f - files.len())
		} else {
			String::new()
		};
		format!("[diff:{}F +{}/-{} {}L | {}{}]", f, a, d, inp.total, files.join(" "), more)
	}
}