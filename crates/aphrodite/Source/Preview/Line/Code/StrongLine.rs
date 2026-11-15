//! Strong code-signature line predicate.
//!
//! This is the predicate the detector trusts on a single occurrence: one such
//! line is enough to classify content as code. It therefore has to stay narrow:
//! a false positive here misroutes prose to the code arm and the model is shown
//! function counts for a paragraph.

use super::{FunctionSignature, IncludeDirective, TypeDeclaration};

/// True for a line that alone is enough to call the content code.
///
/// Matches a function signature in Rust, Python or Go, a type declaration, an
/// `impl`/`class` header, a shebang, a C/C++ include, and a Go `package` line.
/// It is not a language detector - which language the code is written in is a
/// separate question asked elsewhere.
pub(crate) fn Fn(t:&str) -> bool {
	FunctionSignature::Fn(t, "fn ")
		|| FunctionSignature::Fn(t, "def ")
		|| FunctionSignature::Fn(t, "func ")
		|| TypeDeclaration::Fn(t)
		|| t.starts_with("impl ")
		|| t.starts_with("class ")
		|| t.starts_with("#!")
		|| IncludeDirective::Fn(t)
		|| t.strip_prefix("package ")
			.map(|r| r.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false))
			.unwrap_or(false)
}
