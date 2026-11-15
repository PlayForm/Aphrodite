//! An artifact that is at its final path.

use std::path::PathBuf;

/// An artifact that has been moved into place.
///
/// The existence of this value is the whole claim: the bytes are at `Path`, they
/// were verified before they got there, and they carry their final mode.
///
/// What it does **not** claim is that the installation is atomic. Each artifact is
/// published by its own rename, and a rename is atomic for one file, not for a set
/// of them: a failure between the binary's publish and the dylibs' leaves a
/// `binaries/` directory holding one new file and one old one. Setup has no
/// journal and no backup policy, so it cannot roll that back - re-running is the
/// repair, and the plan exists so a caller can see what was going to happen, not
/// so a failed run can be undone.
pub(crate) struct Struct {
	/// The artifact's final path.
	pub Path:PathBuf,
}
