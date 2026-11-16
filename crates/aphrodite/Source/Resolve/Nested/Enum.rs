//! What one occurrence in a document resolved to.

/// The answer for one occurrence the scanner found.
///
/// Three outcomes and three different consequences. Collapsing any two of them is a
/// recorded bug: substituting a placeholder for an unresolvable reference got baked
/// into stored content, so an evicted or late entry could never heal (F1), and
/// treating an abandoned bounded expansion as merely unresolvable would inline a
/// document that still carries markers.
///
/// This is the fourth row of the grammar-tolerance policy, and the only one that is
/// not a grammar question - see `Parse::Tolerance`.
pub(crate) enum Enum {
	/// The reference resolved: substitute this text for the marker.
	Replaced(String),
	/// The store does not hold the hash: leave the ORIGINAL marker text, so a later
	/// resolve of a merely evicted or late entry can still heal.
	Unresolved,
	/// A bounded expansion abandoned its output: the document cannot be completed
	/// within the budget, so nothing may be inlined.
	Abandoned,
}
