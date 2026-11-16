//! The terminal-shape override.

use crate::Hook::Classification::{Hint::Enum::Enum as Hint, Source::Enum::Enum as Source, Struct::Struct};

/// Type terminal output as `terminal` when its own lines make that certain.
///
/// An `exit code:` or `Error:` line is the shell reporting on itself, so the
/// answer needs no detector and no refinement. The override is gated on the
/// hint because the same words in a tool result mean something else: only the
/// terminal path knows that its content is a command's output.
pub(crate) fn Fn<'a>(Content:&'a str, Hint:Hint) -> Option<Struct<'a>> {
	let Certain = matches!(Hint, Hint::Terminal) && (Content.contains("exit code:") || Content.contains("Error:"));

	Certain.then(|| Struct { Type:"terminal".to_string(), Content, Source:Source::TerminalHeuristic })
}
