//! Code shape detector.
//!
//! Code is recognised in two grades: a strong signature line proves it on its
//! own, while statement lines only vote. This detector applies both grades. It
//! is not a language detector - the language is a separate question.

use crate::Preview::{
	Input::Struct::Struct as Input,
	Line::Code::{StrongLine, VoteLine},
};

/// True when the content looks like source code.
///
/// One strong marker (`fn main(`, `def x(`, `struct X`, `#include`, a shebang) or
/// two statement votes (`use x::y;`, `let x =`, `import x`, `return x`) are
/// enough. It is not a syntax check: content that merely quotes code in prose can
/// accumulate two votes, which is why the majority gate exists rather than a
/// single-line test.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let CodeStrong = Inp.NonEmpty.iter().any(|l| StrongLine::Fn(l.trim_start()));
	let CodeVotes = Inp.NonEmpty.iter().filter(|l| VoteLine::Fn(l.trim_start())).count();
	CodeStrong || CodeVotes >= 2
}
