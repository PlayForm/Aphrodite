//! The on-disk file set the embedded directives must produce.
//!
//! Tests assert against this rather than against a literal list of names: the
//! embedded set is the core crate's, so a directive added there must change the
//! expectation here automatically instead of silently failing a hardcoded
//! count.

/// The expected on-disk files: embedded names with the `.md` extension.
///
/// Ordered by name, matching the sort in `Builtin::Fn`, so a test can compare
/// arrays positionally when it needs to.
pub(crate) fn Fn() -> Vec<(String, String)> {
	crate::Directive::Builtin::Fn::Fn()
		.into_iter()
		.map(|Entry| (format!("{}.md", Entry.Name), Entry.Content))
		.collect()
}
