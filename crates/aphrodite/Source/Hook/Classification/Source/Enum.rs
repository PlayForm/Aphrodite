//! How a type was decided.

/// Which step produced the reported content type.
///
/// The classifier used to answer a type and nothing else, so a caller could not
/// tell a caller-supplied type from a detector guess - and a wrong `type` in a
/// preview was untraceable. The source makes the decision auditable without
/// changing the reported string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enum {
	/// The caller supplied the type; it was not re-detected.
	Supplied,
	/// The content carried a line that makes its type certain (`exit code:`,
	/// `Error:`), so the detector was not consulted.
	TerminalHeuristic,
	/// The content detector named the type and nothing refined it.
	Detected,
	/// The detector named a shape and the preview-aware pass upgraded it.
	PreviewRefinement,
}
