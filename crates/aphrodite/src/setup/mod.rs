//! Bootstrap setup: `aphrodite setup` - one-shot install after
//! `cargo install aphrodite`.
//!
//! Creates ~/.hermes/aphrodite/ with binaries, config, and plugin manifest,
//! then optionally launches the proxy. Maximum security: self-verification,
//! permission hardening, no secrets in args.
//!
//! Templates live in `templates/` and are embedded at compile time via
//! `include_str!` - no runtime file dependency for cargo-installed binaries.
//!
//! 1.5.0: atomized into a domain tree: `run.rs` (orchestration + error
//! type + context), `verify.rs` (prereq checks), `dylib.rs` (copy/
//! download/checksum/triple), `macos.rs` (Gatekeeper-safe artifact install),
//! `write.rs` (config/shim/manifest writers + embedded templates), `tests.rs`
//! (battery). This facade re-exports the exact public names the rest of the
//! crate depends on (`setup::run`, `setup::SetupError`).

mod dylib;
#[cfg(target_os = "macos")]
mod macos;
mod run;
mod verify;
mod write;

#[cfg(test)]
mod tests;

pub use run::{SetupError, run};
pub(crate) use run::SetupCtx;
pub(crate) use dylib::copy_dylibs;
#[cfg(test)]
pub(crate) use dylib::verify_download_checksum;
pub(crate) use verify::{self_hash, verify_hermes};
pub(crate) use write::{
	CONFIG_TEMPLATE,
	binary_name,
	register_plugin,
	secure_perms,
	write_binary_version,
	write_init_py,
	write_plugin_yaml,
};
#[cfg(test)]
pub(crate) use write::HERMES_PLUGIN_SHIM;
#[cfg(target_os = "macos")]
pub(crate) use macos::install_macos_artifact;
