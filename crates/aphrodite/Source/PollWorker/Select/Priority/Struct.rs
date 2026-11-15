//! The ordered key an eviction candidate is ranked by.

/// Eviction priority: lower is evicted sooner.
///
/// The derived order compares `Rank` first and `Index` second, so the minimum is
/// the oldest task within the lowest-ranked category - which is exactly the
/// order the old code walked by hand (`position` for a stale task, then for a
/// finished one, then the front of the deque).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Priority {
	/// Category rank: stale, then finished, then running.
	pub Rank:u8,
	/// Insertion index, so the oldest task within a category wins.
	pub Index:usize,
}
