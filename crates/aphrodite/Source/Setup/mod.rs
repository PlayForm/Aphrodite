//! Bootstrap install path: `aphrodite setup`, the one-shot after
//! `cargo install aphrodite`.
//!
//! Setup resolves the Hermes home and the runtime home, installs the binary and
//! both dylibs, writes `aphrodite.toml`, the `plugin.yaml` manifest, the
//! `__init__.py` loader and the `BINARY_VERSION` pin, then registers the plugin
//! with Hermes. It is the only writer of an installed layout, so a silent failure
//! here does not fail setup - it leaves a Hermes runtime that starts, registers,
//! and then cannot load its dylib or its loader, and the symptom appears in the
//! gateway on a later run.
//!
//! The work is grouped by what it touches. `Run/` orchestrates and owns the error
//! type and the resolved context; `Verify/` checks prerequisites and hashes the
//! artifact; `Dylib/` locates, downloads and checksums the shared libraries;
//! `Write/` writes every installed file, the two `include_str!` templates
//! included; `MacOS.rs` applies the Gatekeeper-safe treatment to each artifact copy
//! on macOS.
//!
//! This facade declares the leaves and carries no logic. `Run::Entry::Fn` and
//! `Run::Error::Enum` are the only items anything outside `Setup/` names, and
//! `Run` is `pub` because the binary - a separate crate target - calls it.

mod Dylib;
#[cfg(target_os = "macos")]
mod MacOS;
pub mod Run;
mod Verify;
mod Write;

#[cfg(test)]
mod Test;
