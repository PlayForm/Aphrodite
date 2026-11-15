//! One downloaded artifact, before anything has checked it.
//!
//! The four `Artifact` types are the download pipeline as values:
//! `Staged` -> `Verified` -> `Prepared` -> `Published`. Each step consumes the
//! previous value and produces the next, so a file cannot be published without
//! having been verified, and it cannot be verified without having been staged.

use std::path::PathBuf;

/// Bytes on disk that no one has vouched for yet.
///
/// `Path` is a staging path on the **destination's own filesystem** - a sibling of
/// `Dest` with a `.partial` suffix - not a temporary directory. That placement is
/// the contract: publication is a rename, and a rename does not cross mount
/// points, so staging in `$TMPDIR` would make the publish step a copy and lose the
/// property that a half-written file never appears at the final name.
///
/// `Dest` is carried so the later steps do not have to be told it again, and so a
/// `Staged` value names both ends of the move it is going to take part in.
pub(crate) struct Struct {
	/// Where the bytes are now.
	pub Path:PathBuf,
	/// Where they are going, once they are verified and prepared.
	pub Dest:PathBuf,
}
