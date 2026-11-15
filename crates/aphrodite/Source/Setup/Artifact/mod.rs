//! Downloading a release asset into a staging path.
//!
//! The pipeline is `Stage` -> `Verify` -> `Prepare` -> `Publish`, one value per
//! step. Nothing is published before it is verified, and nothing is verified at the
//! destination: the staged file is a sibling of its destination with a `.partial`
//! suffix, so the last step is a rename on one filesystem.

pub mod Prepare;
pub mod Prepared;
pub mod Publish;
pub mod Published;
pub mod Stage;
pub mod Staged;
pub mod Verify;
pub mod Verified;
