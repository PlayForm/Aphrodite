//! A bounded sample that still counts everything.

/// The first `N` items of a sequence, plus the length of the whole sequence.
///
/// A reducer has to report an exact count and show a bounded example set, and the previous
/// implementation collected EVERY matching line into a `Vec` and then threw most of it away with
/// `take`. This type keeps the count exact while the retained memory is `N` items, and the `Vec`
/// is never pre-allocated, so a sequence that matches nothing allocates nothing.
///
/// It is not a reservoir sample: the first `N` in input order are what the renderer shows, which
/// is what the previous `take(n)` produced.
pub(crate) struct Struct<T, const N: usize> {
	Sample:Vec<T>,
	Total:usize,
}

impl<T, const N: usize> Struct<T, N> {
	pub(crate) fn new() -> Self { Self { Sample:Vec::new(), Total:0 } }

	/// Count one item and retain it while there is room.
	///
	/// The count moves for every item; the sample moves only for the first `N`. It is not a
	/// filter: an item beyond the cap is still counted, which is the whole reason this type
	/// exists instead of `take`.
	pub(crate) fn Push(&mut self, Item:T) {
		self.Total += 1;

		let _ = (self.Sample.len() < N).then(|| self.Sample.push(Item));
	}

	/// How many items were offered, including the ones not retained.
	pub(crate) fn Total(&self) -> usize { self.Total }

	/// True when nothing was offered.
	pub(crate) fn IsEmpty(&self) -> bool { self.Total == 0 }

	/// The retained examples, in input order.
	pub(crate) fn IntoSample(self) -> Vec<T> { self.Sample }
}
