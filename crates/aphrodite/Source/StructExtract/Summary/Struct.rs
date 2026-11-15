//! The ordered categories one extraction produced.

use std::collections::HashMap;

/// The categories a language produced, in priority order, with their signature lines.
///
/// The order is retained while the summary is being built and lost only at the boundary, where the
/// public shape is a `HashMap` - which is what every caller already consumes. An empty category is not
/// inserted, so an absent key means "none found" rather than "not asked for", exactly as before.
///
/// It is not a budget: the budget is spent by the scanner, and a category that ran out of room arrives
/// here already empty.
pub(crate) struct Struct {
	Categories:Vec<(String, Vec<String>)>,
}

impl Struct {
	pub(crate) fn new() -> Self { Self { Categories:Vec::new() } }

	/// Record one category, skipping it when it produced nothing.
	///
	/// The key is the wire category (`fns`, `structs`, `traits`, `impls`, `classes`, `types`) and is
	/// carried by the policy rather than by the recogniser, so a recogniser cannot invent a category.
	pub(crate) fn Push(&mut self, Key:&str, Lines:Vec<String>) {
		let _ = (!Lines.is_empty()).then(|| self.Categories.push((Key.to_string(), Lines)));
	}

	/// The public shape: category key to signature lines.
	pub(crate) fn IntoMap(self) -> HashMap<String, Vec<String>> { self.Categories.into_iter().collect() }
}
