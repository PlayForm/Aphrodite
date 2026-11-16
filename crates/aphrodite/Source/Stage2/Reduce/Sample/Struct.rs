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
	/// The item is BUILT only when there is room, because the caller hands over a closure rather
	/// than a value. That is the difference between bounding the retention and bounding the work:
	/// a reducer that formatted every match and then dropped all but the first `N` still paid the
	/// allocation, the truncation and the lowercase scan for a log of a million lines.
	///
	/// The count moves for every item; the sample moves only for the first `N`. It is not a
	/// filter: an item beyond the cap is still counted, which is the whole reason this type
	/// exists instead of `take`.
	pub(crate) fn Push(&mut self, Item:impl FnOnce() -> T) {
		self.Total += 1;

		// `then` keeps the item unbuilt when the sample is already full.
		let Room = self.Sample.len() < N;

		self.Sample.extend(Room.then(Item));
	}

	/// How many items were offered, including the ones not retained.
	pub(crate) fn Total(&self) -> usize { self.Total }

	/// True when nothing was offered.
	pub(crate) fn IsEmpty(&self) -> bool { self.Total == 0 }

	/// The retained examples, in input order.
	pub(crate) fn IntoSample(self) -> Vec<T> { self.Sample }
}
