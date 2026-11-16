//! A file's content and everything derived from it, before the lock is taken.

use crate::State::Identity::Content::Struct::Struct as Digest;

/// One file's content plus the facts the store, the marker and the report need.
///
/// Classification, hashing and preview construction all live here so they can run
/// BEFORE the state is borrowed: they are pure functions of the bytes, and doing them
/// inside the insertion phase held whatever lock guards the session across work that
/// needed nothing from it.
///
/// `Size` is the length of the content actually read, which is what is stored and what
/// the marker and the report quote. It is deliberately NOT the size the filesystem
/// reported: a file that changed between the size check and the read has a different
/// length, and quoting the stat size would put a number in the model's marker that
/// describes a payload the store does not hold.
#[derive(Debug, Clone)]
pub(crate) struct Prepared {
	/// The path the content came from.
	pub Path:String,
	/// The content itself, read in full.
	pub Content:String,
	/// Content identity: the CCR hash of the bytes.
	pub Hash:Digest,
	/// Detected content type, as the marker and the report name it.
	pub Type:String,
	/// The preview built from the content.
	pub Preview:String,
	/// The length of the content actually read.
	pub Size:usize,
}
