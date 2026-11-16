//! The one truncation every preview passes through.

use std::num::NonZeroUsize;

use crate::Preview::Limit::Enum::Enum as Limit;

/// Smallest cap that can hold the ellipsis and the closing bracket together: four.
const Floor:usize = 4;

/// Characters the closing bracket and the ellipsis occupy together: four.
const Reserve:usize = 4;

/// Truncate a preview to its budget, on a character boundary.
///
/// Preserves the closing `]` of a self-bracketed preview (with a `...` marker before it), because a
/// bare cut that drops `]` re-triggers the `[text:[text:...]]` double-wrap bug class - the marker
/// parser and the chain splitter both read that bracket. It is not a byte truncation: the cut is by
/// `char`, because a cut inside a multi-byte character would panic.
///
/// A preview already inside its budget is handed back untouched, so the common case does not
/// allocate at all: the caller always owns the string, and borrowing it only to `into_owned` it
/// copied every fitting preview.
///
/// "Fits" means `count <= Maximum`. The over-budget case is the `NonZeroUsize` of the excess, not
/// a boolean, so the boundary is decided by arithmetic that cannot be off by one: at equality the
/// excess is zero and the preview is returned unchanged. A `checked_sub` here reads equality as a
/// successful subtraction and truncates a preview that already fits.
pub(crate) fn Fn(Preview:String, Limit:&Limit) -> String {
	let Maximum = match Limit {
		Limit::Unlimited => return Preview,
		Limit::Characters(Maximum) => Maximum.get(),
	};

	match NonZeroUsize::new(Preview.chars().count().saturating_sub(Maximum)) {
		None => Preview,
		Some(_) => Cut(&Preview, Maximum),
	}
}

/// The cut itself, for a preview that is over budget.
///
/// The bracket-preserving branch is chosen by an `Option`, not by a boolean: `Maximum - 4` is only
/// meaningful when the bracket is kept, and computing it eagerly would underflow for a cap below four.
fn Cut(Preview:&str, Maximum:usize) -> String {
	let KeepClose = Preview.trim_end().ends_with(']');

	match (KeepClose && Maximum >= Floor).then(|| Maximum - Reserve) {
		None => Preview.chars().take(Maximum).collect(),
		Some(Room) => {
			let mut Out:String = Preview.chars().take(Room).collect();

			Out.push_str("...");
			Out.push(']');

			Out
		},
	}
}
