//! Bytes whose checksum matched the release's manifest.
//!
//! A `Verified` can only be produced by `Verify::Fn`, and `Prepare::Fn` takes one,
//! so the checksum cannot be skipped by ordering the steps differently: there is no
//! path from `Staged` to `Prepared`.

use super::super::Staged;

/// A staged artifact whose bytes match the release's `SHA256SUMS-<triple>.txt`.
///
/// The value carries the whole `Staged` rather than just a path, so the staging
/// path and the destination stay together and the later steps cannot be handed one
/// without the other.
pub(crate) struct Struct {
	/// The artifact that was verified.
	pub Staged:Staged::Struct::Struct,
}
