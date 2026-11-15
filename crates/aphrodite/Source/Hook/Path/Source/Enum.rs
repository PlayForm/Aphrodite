//! Where a resolved path came from.

/// Which evidence named a path.
///
/// It is carried because the two sources are not equally trustworthy and feed
/// different signals: an argument is what the agent asked for, while output is
/// a heuristic over the first line of what came back. A caller that could not
/// tell them apart would treat a guess as a declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Enum {
	/// The call's own arguments named it, under this key.
	Argument { Key:&'static str },
	/// The first line of the call's output looked like a path.
	Output,
}
