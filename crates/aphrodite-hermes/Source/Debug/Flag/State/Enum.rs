//! What a flag file said, including "there is no flag file".

/// The state of one flag file.
///
/// Three states, because absence is not a value: a session nobody toggled has no
/// flag file at all, a session toggled off has a file that says so, and the
/// resolution walks a chain where "keep looking at the parent" is a different
/// answer from "this root said off". A reader that answered `bool` folded all
/// three into `false`, which is why the resolver needed a separate `exists()`
/// stat to tell them apart - one syscall duplicated, and a race between the two.
///
/// `Copy` because the mtime cache stores it and hands it back: the enum is
/// fieldless, so copying it is the whole of the cost.
#[derive(Clone, Copy)]
pub(crate) enum Enum {
	/// The file exists and its leading token means enabled.
	On,
	/// The file exists and its leading token means disabled.
	Off,
	/// No file at this path. The normal state for a session nobody toggled.
	Absent,
}
