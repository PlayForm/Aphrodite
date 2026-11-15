//! What a completed task's output is called and where it goes.
//!
//! The output is owned here, not borrowed: it was moved out of the task's
//! accumulator by the poll that completed it, so nothing copies a quarter megabyte
//! to hand it to the store.

/// A completed poll worker's output, as the store and the marker need it.
#[derive(Debug, Clone)]
pub(crate) struct Completion {
	/// CCR hash of the stored output.
	pub Hash:String,
	/// The stored output itself.
	pub Output:String,
	/// Preview built from it, as the marker and the completion nudge show it.
	pub Preview:String,
	/// Byte size of the stored output.
	pub Size:usize,
	/// The command line the marker's centre names.
	pub Command:String,
	/// The exit code that finished the task.
	pub ExitCode:i32,
}
