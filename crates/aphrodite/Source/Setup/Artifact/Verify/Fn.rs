//! Proving a staged artifact against the release's checksum manifest.

use crate::Setup::Dylib::Checksum;
use super::super::{Staged, Verified};

/// Verify a staged artifact and hand it on as `Verified`.
///
/// The sums file is fetched from the same release directory the asset came from, so
/// a mirror cannot serve bytes the release does not vouch for. Both the file and
/// the asset's entry in it are mandatory.
///
/// It is not a best-effort check and it has no "skip verification" path: a missing
/// sums file, a missing entry, or a hash mismatch all return `Err`, which makes
/// setup exit non-zero. The staged file is deleted on rejection, because leaving it
/// behind would let a later step install bytes that were never verified.
///
/// Taking `Staged` by value and returning `Verified` is what makes the order
/// structural: there is no function that produces a `Verified` from anything else,
/// so no caller can reach `Prepare` with unverified bytes.
pub(crate) fn Fn(
	Staged:Staged::Struct::Struct,
	ReleaseDirectory:&str,
	Triple:&str,
	AssetName:&str,
) -> anyhow::Result<Verified::Struct::Struct> {
	match Checksum::Fn(ReleaseDirectory, Triple, AssetName, &Staged.Path) {
		Ok(()) => Ok(Verified::Struct::Struct { Staged }),
		Err(e) => {
			let _ = std::fs::remove_file(&Staged.Path);

			Err(e.context(format!("verify the checksum of {AssetName}")))
		},
	}
}
