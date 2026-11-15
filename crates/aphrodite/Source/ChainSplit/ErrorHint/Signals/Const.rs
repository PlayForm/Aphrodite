//! The words that make a line an error signal.

/// Lower-case substrings whose presence marks a line as an error signal.
///
/// It is a vocabulary, not a grammar: the hint exists so the model learns
/// "that segment failed" from the segment's own words, so the list stays
/// content-level and never names a mechanism. `exit code:` is checked
/// separately and case-sensitively, because that exact spelling is what a
/// shell prints.
pub(crate) const Signals:&[&str] = &[
	"error",
	"failed",
	"failure",
	"panic",
	"traceback",
	"fatal",
	"cannot",
	"not found",
	"no such file",
	"permission denied",
	"unresolved import",
	"undefined",
];
