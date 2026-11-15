//! `Base::Fn`: the flat defaults plus the content-derived values.

use std::collections::HashMap;

use crate::Template::{
	Extract::{
		Command::Fn::Fn as ExtractCommand,
		Commit::Fn::Fn as ExtractCommit,
		Diff::Fn::Fn as DiffCounts,
		Exit::Fn::Fn as ExtractExit,
	},
	Vars::Stat::Fn::Fn as Stat,
};

/// The flat half of the variable set: every name the pre-Rust template
/// namespace exposes, defaulted, then overwritten with whatever the content
/// actually yields.
///
/// The defaults are the contract, not filler: a template that names `Files` or
/// `pid` on content that has neither must render `"?"`, which is what the
/// pre-Rust engine did. Replacing a default with `""` changes every rendered
/// preview that uses it, so they are kept verbatim.
pub(crate) fn Fn(TypeStr:&str, Content:&str) -> HashMap<String, String> {
	let Lines:Vec<&str> = Content.lines().collect();
	let Ln = Lines.len();
	let Lower:Vec<String> = Lines.iter().map(|l| l.to_ascii_lowercase()).collect();

	let Stats = Stat(Content, &Lines, &Lower);
	let First = Stats.FirstLine;
	let ErrorCount = Stats.ErrorCount;
	let WarnCount = Stats.WarningCount;
	let Message = Stats.Message;
	let Exit = ExtractExit(&Lines);
	let Command = ExtractCommand(&Lines);
	let Commit = ExtractCommit(&Lines);
	let (Plus, Minus) = DiffCounts(&Lines);

	let mut Vars:HashMap<String, String> = HashMap::new();
	Vars.insert("type".into(), TypeStr.to_string());
	Vars.insert("ln".into(), Ln.to_string());
	Vars.insert("size".into(), Content.len().to_string());
	Vars.insert("first".into(), First);
	Vars.insert("err".into(), ErrorCount.to_string());
	Vars.insert("warn".into(), WarnCount.to_string());
	Vars.insert("msg".into(), Message);
	Vars.insert("exit".into(), Exit);
	Vars.insert("cmd".into(), Command.clone());
	Vars.insert("files".into(), "?".into());
	Vars.insert("fn".into(), String::new());
	Vars.insert("plus".into(), Plus);
	Vars.insert("minus".into(), Minus);
	Vars.insert("keys".into(), String::new());
	Vars.insert("items".into(), "?".into());
	Vars.insert("commit".into(), Commit.clone());
	Vars.insert("hash".into(), Commit);
	Vars.insert("subject".into(), String::new());
	Vars.insert("pid".into(), "?".into());
	Vars.insert("uptime".into(), String::new());
	// Code-structure defaults (pre-Rust): fns "?", structs/impls/classes "0".
	Vars.insert("fns".into(), "?".into());
	Vars.insert("structs".into(), "0".into());
	Vars.insert("impls".into(), "0".into());
	Vars.insert("classes".into(), "0".into());
	Vars.insert("sigs".into(), String::new());
	// Pre-Rust extras + computed vars.
	Vars.insert("code".into(), "?".into());
	Vars.insert("loc".into(), String::new());
	Vars.insert("errors".into(), ErrorCount.to_string());
	Vars.insert("total".into(), Ln.to_string());
	Vars
}
