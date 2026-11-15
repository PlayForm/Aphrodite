//! The one truncation every preview passes through.

use std::borrow::Cow;

use crate::Preview::Limit::Enum::Enum as Limit;

/// Smallest cap for which the bracket-preserving cut is still possible.
const BracketFloor:usize = 3;

/// Characters the closing bracket and the ellipsis occupy together.
const BracketAndEllipsis:usize = 4;

/// Truncate a preview to its budget, on a character boundary.
///
/// Preserves the closing `]` of a self-bracketed preview (with a `...` marker before it), because a
/// bare cut that drops `]` re-triggers the `[text:[text:...]]` double-wrap bug class - the marker
/// parser and the chain splitter both read that bracket. It is not a byte truncation: the cut is by
/// `char`, because a cut inside a multi-byte character would panic.
///
/// A preview already inside its budget is returned borrowed, so the common case allocates nothing.
/// The over-budget test is `checked_sub`, which makes "it fits" a `None` rather than a boolean the
/// caller has to branch on.
pub(crate) fn Fn<'a>(Preview:&'a str, Limit:&Limit) -> Cow<'a, str> {
	let Maximum = match Limit {
		Limit::Unlimited => return Cow::Borrowed(Preview),
		Limit::Characters(Maximum) => Maximum.get(),
	};

	match Preview.chars().count().checked_sub(Maximum) {
		None => Cow::Borrowed(Preview),
		Some(_) => Cow::Owned(Cut(Preview, Maximum)),
	}
}

/// The cut itself, for a preview that is over budget.
///
/// The bracket-preserving branch is chosen by an `Option`, not by a boolean: `Maximum - 4` is only
/// meaningful when the bracket is kept, and computing it eagerly would underflow for a cap below four.
fn Cut(Preview:&str, Maximum:usize) -> String {
	let KeepClose = Preview.trim_end().ends_with(']');

	match (KeepClose && Maximum > BracketFloor).then(|| Maximum - BracketAndEllipsis) {
		None => Preview.chars().take(Maximum).collect(),
		Some(Room) => {
			let mut Out:String = Preview.chars().take(Room).collect();

			Out.push_str("...");
			Out.push(']');

			Out
		},
	}
}
