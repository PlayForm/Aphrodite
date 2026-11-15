//! The content-derived half of the variable set.

use std::collections::HashMap;

use crate::Template::{
	Extract::{
		Command::Fn::Fn as ExtractCommand,
		Commit::Fn::Fn as ExtractCommit,
		Diff::Fn::Fn as ExtractDiff,
		Exit::Fn::Fn as ExtractExit,
	},
	Vars::Stat::Fn::Fn as Stat,
};

/// Fill the variables that come out of the content itself.
///
/// This is the ONE place a "not found" placeholder is spelled. The extractors answer
/// `None`, so "no commit in this content" and "a commit whose hash is `???????`"
/// stopped being the same value, and the pre-Rust rendering of absence is applied
/// here - once per variable, where the template namespace is defined, instead of
/// inside each scanner.
///
/// `Lower` is passed in rather than recomputed so the caller pays for the lowercased
/// copy once: it is the same vector the message search walks.
pub(crate) fn Fn(Vars:&mut HashMap<String, String>, Content:&str, Lines:&[&str], Lower:&[String]) {
	let Stats = Stat(Content, Lines, Lower);

	Vars.insert("first".into(), Stats.FirstLine);
	Vars.insert("err".into(), Stats.ErrorCount.to_string());
	Vars.insert("warn".into(), Stats.WarningCount.to_string());
	Vars.insert("msg".into(), Stats.Message);
	Vars.insert("errors".into(), Stats.ErrorCount.to_string());

	Vars.insert("exit".into(), ExtractExit(Lines).unwrap_or_else(|| "?".to_string()));
	Vars.insert("cmd".into(), ExtractCommand(Lines).unwrap_or_default());

	// The hash renders under two names and the placeholder is one string, so the
	// value is built once and cloned for the second key.
	let Commit = ExtractCommit(Lines).unwrap_or_else(|| "???????".to_string());
	Vars.insert("commit".into(), Commit.clone());
	Vars.insert("hash".into(), Commit);

	// `?`/`?` is the "not a diff" answer and `0`/`0` the "a diff that changed
	// nothing" one; the record carries the counts and this arm carries the
	// distinction.
	let (Plus, Minus) = match ExtractDiff(Lines) {
		Some(Diff) => (Diff.Plus.to_string(), Diff.Minus.to_string()),
		None => ("?".to_string(), "?".to_string()),
	};
	Vars.insert("plus".into(), Plus);
	Vars.insert("minus".into(), Minus);
}
