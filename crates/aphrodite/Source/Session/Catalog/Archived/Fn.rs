//! The archived-turns line.

/// The archived-turn count, when there is anything archived.
///
/// A count only, not a delta: the index changes slowly and the line is about
/// forty characters, so it is worth re-stating every turn - it is the model's only
/// signal that `aphrodite_diff` has something to return.
pub(crate) fn Fn(Count:usize) -> Option<String> {
	(Count > 0).then(|| format!("{Count} archived turns available for retrieval."))
}
