//! The fresh half of the walk: enter the path, expand, leave it.

use crate::Resolve::{Expansion, Halt, Traversal::Struct::Struct as Traversal};
use super::Build;

/// Expand `HashValue` and every marker its content contains.
///
/// Called only for a hash the prelude classified `Fresh`, so the cycle, depth and cache
/// questions are already settled and are not repeated here.
///
/// The path is entered BEFORE the content is loaded and left whatever the expansion finished
/// with - a completed expansion, an absent hash and an abandoned budget all leave through the
/// same door. That is why entering lives here rather than in the prelude: a hash that is
/// entered must be left, and the two statements belong where a reader sees them together. A
/// path that was never popped was the old shape, and it made "already on the path" and "seen
/// anywhere in this tree" the same test - so a hash first reached at the depth cap stayed
/// capped for every later branch.
pub(crate) fn Fn(
	Traversal:&mut Traversal,
	HashValue:&str,
	Depth:usize,
) -> Result<Expansion::Enum::Enum, Halt::Enum::Enum> {
	Traversal.Enter(HashValue);

	let Out = Build::Fn::Fn(Traversal, HashValue, Depth);

	Traversal.Leave();

	Out
}
