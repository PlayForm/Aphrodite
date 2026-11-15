//! The size regimes, as a value.
//!
//! Three regimes, not two: content above the compression threshold is stored
//! and replaced by a marker; content below it but above the inline threshold is
//! stored only so a later retrieval finds it without a backend round-trip; the
//! rest is left alone. Expressed as two ordered comparisons it was a nested
//! `match` on `()` whose arms each re-read the same lengths.

/// Which regime a message's content falls into.
pub(crate) enum Enum {
	/// Above the compression threshold: store and replace with a marker.
	Compress,
	/// Above the inline threshold only: store, leave the wire response alone.
	Inline,
	/// Below both: nothing to do.
	Skip,
}
