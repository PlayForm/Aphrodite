//! Which transform path is classifying.
//!
//! The same content means different things on the two paths: an `Error:` line in
//! a terminal result is the shell reporting on itself, while the same words in a
//! tool result are just text. The hint names the path, so the override cannot be
//! enabled by a bare `true` at a call site that says nothing about why.

/// The transform path a classification was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enum {
	/// A tool result: the content is whatever the tool produced.
	Tool,
	/// Terminal output: the content is a command's output, so shell-shaped
	/// lines are certain rather than suggestive.
	Terminal,
}
