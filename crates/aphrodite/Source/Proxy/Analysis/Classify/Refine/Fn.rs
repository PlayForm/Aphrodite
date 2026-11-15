//! Turn the pipeline's generic `code` into a language-specific name.
//!
//! The pipeline classifies shape, not language, so every code block arrived as
//! `code` and the threshold scaler could not tell a Rust file from a shell
//! script. The language pass is the only thing consulted for this one name; a
//! block it cannot place stays `code`, which is what the `.unwrap_or("code")`
//! at the old call site did.

/// Refine a detected name using the content it was detected from.
pub(crate) fn Fn(Name:&'static str, Content:&str) -> &'static str {
	match Name {
		"code" => crate::Preview::Language::Fn::Fn(Content).unwrap_or("code"),
		Other => Other,
	}
}
