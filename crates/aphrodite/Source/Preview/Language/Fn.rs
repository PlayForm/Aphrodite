//! Per-language code refinement.
//!
//! The shape detector can only say "this is code". Which language it is decides which type
//! string the marker carries, and that in turn decides whether the model knows the payload is
//! Rust or Python. The rules are re-expressed line-based from the old proxy classifier so the
//! proxy path and the hook/FFI path classify identically. It is not a language detector in
//! general - it only refines content already classified as code.

use super::{Go, JS, Python, Rust};

/// Lines a snippet needs before a language rule runs: three. Below it the shape is
/// `code` and no language is named.
const Minimum:usize = 3;

/// The language rules, in precedence order.
///
/// The order IS the precedence: the first rule that matches names the language, so reordering
/// this array reorders the classification. It is not a score - no rule is weighted against
/// another.
/// The shape of one rule: the type string it names, and its predicate.
type Rule = fn(&str) -> bool;

const Rules:[(&str, Rule); 4] = [
	("code_rust", Rust::Fn::Fn),
	("code_python", Python::Fn::Fn),
	("code_go", Go::Fn::Fn),
	("code_js", JS::Fn::Fn),
];

/// Refine a `"code"` shape into a language-specific type, or `None`.
///
/// Content of three lines or fewer returns `None` without running a rule, because a snippet
/// that short carries no language evidence and guessing one would put a wrong language in the
/// marker. It is not a parser and it is not a tie-break: the first matching rule wins.
pub(crate) fn Fn(Content:&str) -> Option<&'static str> {
	(Content.lines().count() > Minimum)
		.then_some(Content)
		.and_then(|Content| Rules.into_iter().find(|(_, Rule)| Rule(Content)).map(|(Name, _)| Name))
}
