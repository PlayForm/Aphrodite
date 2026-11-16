//! Code statement-line vote predicate.
//!
//! Statements are the weaker grade of code evidence: any one of them could
//! appear in prose or in shell output, so the detector counts them and needs a
//! majority rather than a single hit. This predicate is that vote.

use super::{FromImport, LetAssignment, UseStatement};

/// True for a line that votes for the content being code.
///
/// Covers `use x::y;`, `let x =`, `import x`, `from x import y`, `return ...`,
/// `println!`, `print(` and `echo ...`. It is not proof on its own - the
/// detector requires a majority of such lines before it classifies content as
/// code, precisely because a single `return` appears in ordinary prose.
pub(crate) fn Fn(t:&str) -> bool {
	UseStatement::Fn(t)
		|| LetAssignment::Fn(t)
		|| FromImport::Fn(t)
		|| t.strip_prefix("import ")
			.map(|r| r.chars().next().map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false))
			.unwrap_or(false)
		|| t.strip_prefix("return")
			.map(|r| r.chars().next().map(|c| c.is_whitespace()).unwrap_or(false))
			.unwrap_or(false)
		|| t.starts_with("println!")
		|| t.starts_with("print(")
		|| t.strip_prefix("print")
			.map(|r| r.trim_start().starts_with('('))
			.unwrap_or(false)
		|| t.strip_prefix("echo ")
			.map(|r| r.chars().next().map(|c| c.is_alphanumeric() || c == '$').unwrap_or(false))
			.unwrap_or(false)
}
