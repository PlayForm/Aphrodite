//! Classify a terminal output into its CCR type.

use crate::Tool::Unwrap::Matcher::Terminal::Classify::{Kind, Resolve};

/// The CCR type for a non-empty terminal output.
///
/// It renders the resolved kind and decides nothing: `Resolve::Fn::Fn` owns the
/// order, and this match is total over that value, so a new kind cannot reach
/// the marker without a label here.
pub(crate) fn Fn(Output:&str) -> String {
	match Resolve::Fn::Fn(Output) {
		Kind::Enum::Failure => "terminal".to_string(),
		Kind::Enum::Build { CompilerError: true } => "build_error".to_string(),
		Kind::Enum::Build { CompilerError: false } => "build_output".to_string(),
		Kind::Enum::Detected => aphrodite::Preview::Detect::Type::Fn(Output),
	}
}
