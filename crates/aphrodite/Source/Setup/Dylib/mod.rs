//! Locating and installing the shared libraries setup ships.
//!
//! The Hermes plugin loads `libaphrodite_hermes`; the core `libaphrodite` exists
//! for external embedders and is best-effort. `Copy/` owns the search-and-install
//! pass, `Download/` fetches a missing asset from the matching GitHub release,
//! `Checksum/` proves the fetched bytes against `SHA256SUMS-<triple>.txt`, and
//! `TargetTriple/` names the asset.
//!
//! The split is not cosmetic: the strategies have different failure rules. A
//! locally absent core dylib is skipped with a warning, an absent Hermes dylib is
//! fatal, and an unverifiable download is refused either way.

pub mod Checksum;
pub mod Copy;
pub mod Download;
pub mod TargetTriple;
