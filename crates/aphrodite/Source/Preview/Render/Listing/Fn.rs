//! Render the directory-listing tally.

use crate::Preview::Summary::Listing::Struct::Struct as Summary;

/// Extensions a listing preview shows: three.
const ExtensionsShown:usize = 3;

/// Render a listing summary as `[ls:<n> files <n> dirs | <ext>×<n> ...]`.
///
/// The extension block is omitted when no entry carried an extension, which is why the two shapes
/// differ only by their suffix. A listing that recognised nothing renders as a bare line count. It is
/// not a tree: no depth, no sizes, no permissions.
pub(crate) fn Fn(Summary:&Summary, Total:usize) -> String {
	(!Summary.IsEmpty())
		.then_some(Summary)
		.map_or_else(|| format!("[ls:{Total}L]"), Body)
}

/// The counts and extensions of a listing that recognised at least one entry.
fn Body(Summary:&Summary) -> String {
	let Extensions:Vec<String> = Summary
		.Extensions(ExtensionsShown)
		.into_iter()
		.map(|(Extension, Count)| format!("{Extension}×{Count}"))
		.collect();

	match Extensions.as_slice() {
		[] => format!("[ls:{} files {} dirs]", Summary.Files(), Summary.Directories()),
		_ => {
			format!(
				"[ls:{} files {} dirs | {}]",
				Summary.Files(),
				Summary.Directories(),
				Extensions.join(" ")
			)
		},
	}
}
