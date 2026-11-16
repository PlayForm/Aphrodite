//! What a tool-result transform was asked to do.

use crate::Hook::Meta::Struct::Struct as Call;

/// The input of one tool-result transform.
///
/// `Meta` is borrowed for the length of one call - every field points into the
/// caller's JSON or string slice - so holding it here costs nothing and keeps the
/// observation and the decision reading the same facts.
///
/// `Copy` because every field is a borrowed slice: the plan carries the input by
/// value, and selection has to read the content and the tool name while building
/// that plan.
#[derive(Debug, Clone, Copy)]
pub struct Struct<'a> {
	/// The result to transform, exactly as the tool produced it.
	pub Content:&'a str,
	/// The tool that produced it.
	pub Tool:&'a str,
	/// Caller-supplied `(content_to_classify, type)`, when the caller knows.
	pub Classify:Option<(&'a str, &'a str)>,
	/// Hermes-supplied call metadata.
	pub Meta:&'a Call<'a>,
}
