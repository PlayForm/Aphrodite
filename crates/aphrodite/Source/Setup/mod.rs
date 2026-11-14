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
//! crate depends on (`Setup::run`, `Setup::SetupError`).

mod Dylib;
#[cfg(target_os = "macos")]
mod Macos;
mod Run;
mod Verify;
mod Write;

#[cfg(test)]
mod Test;

pub use Run::{SetupError, run};
pub(crate) use Run::SetupCtx;
pub(crate) use Dylib::copy_dylibs;
#[cfg(test)]
pub(crate) use Dylib::verify_download_checksum;
pub(crate) use Verify::{self_hash, verify_hermes};
pub(crate) use Write::{
	CONFIG_TEMPLATE,
	binary_name,
	register_plugin,
	secure_perms,
	write_binary_version,
	write_init_py,
	write_plugin_yaml,
};
#[cfg(test)]
pub(crate) use Write::HERMES_PLUGIN_SHIM;
#[cfg(target_os = "macos")]
pub(crate) use Macos::install_macos_artifact;
