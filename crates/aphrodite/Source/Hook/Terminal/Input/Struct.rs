//! What a terminal transform was asked to do.

/// The input of one terminal-output transform.
///
/// `Classify` is the caller's optional `(content_to_classify, type)` pair: a
/// bridge that has already found the real payload underneath a wrapper may say
/// what shape it is, and core stays agnostic to what the wrapper looks like. The
/// command and the exit code are deliberately absent - they belong to the
/// observation, not to the decision, and a plan that could read them would be
/// tempted to branch on them.
pub struct Struct<'a> {
	/// The output to transform, exactly as the call produced it.
	pub Content:&'a str,
	/// Caller-supplied `(content_to_classify, type)`, when the caller knows.
	pub Classify:Option<(&'a str, &'a str)>,
}
