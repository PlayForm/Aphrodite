//! The label set an entry renders after its name.

/// The label set an entry renders after its name.
#[derive(Clone, Copy)]
pub enum Enum {
	/// No labels at all.
	Plain,
	/// One label with a fixed value - a wire contract that names its value
	/// literally, so it is written literally here too.
	Literal {
		Key:&'static str,
		Value:&'static str,
	},
	/// One label whose value is read from the stats document, with the value to
	/// use when the document does not hold it.
	Document {
		Key:&'static str,
		Path:&'static [&'static str],
		Fallback:&'static str,
	},
}
