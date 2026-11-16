//! Bootstrap install path: `aphrodite setup`, the one-shot after
//! `cargo install aphrodite`.
//!
//! Setup resolves the Hermes home and the runtime home, installs the binary and both
//! dylibs, writes `aphrodite.toml`, the `plugin.yaml` manifest, the `__init__.py`
//! loader and the `BINARY_VERSION` pin, then registers the plugin with Hermes. It is
//! the only writer of an installed layout, so a silent failure here does not fail
//! setup - it leaves a Hermes runtime that starts, registers, and then cannot load its
//! dylib or its loader, and the symptom appears in the gateway on a later run.
//!
//! The work is grouped by what it touches, and by when:
//!
//! - `Plan/` decides what will happen - both homes, the catalog-install boundary, the
//!   ordered step list - and writes nothing;
//! - `Execute/` performs that list and returns a report of what completed;
//! - `Artifact/` is the download pipeline as four values: staged, verified, prepared,
//!   published, in that order and not any other;
//! - `Register/` runs the registration command and returns an outcome value;
//! - `Verify/` checks prerequisites and hashes the artifact;
//! - `Dylib/` locates, downloads and checksums the shared libraries;
//! - `Write/` writes every installed file, the two `include_str!` templates included;
//! - `MacOS/` applies the Gatekeeper-safe treatment to each artifact copy on macOS.
//!
//! `Run::Entry::Fn` is the only item anything outside `Setup/` names, and `Run` is
//! `pub` because the binary - a separate crate target - calls it. The rest of the tree
//! is `pub(crate)`: it is structure, not surface.

pub(crate) mod Artifact;
mod Dylib;
pub(crate) mod Execute;
#[cfg(target_os = "macos")]
mod MacOS;
pub(crate) mod Plan;
pub(crate) mod Register;
pub mod Run;
mod Verify;
mod Write;

#[cfg(test)]
mod Test;
