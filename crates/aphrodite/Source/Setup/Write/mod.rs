//! Every file `aphrodite setup` installs.
//!
//! Two leaves are `include_str!` constants - the `aphrodite.toml` template and the
//! Hermes `__init__.py` loader - so a cargo-installed binary carries its install
//! payload and needs no source checkout at run time. The rest write one artifact
//! each into the directory that owns it: configuration and the version pin into the
//! runtime home, the loader and manifest into the hooks-only Hermes plugin
//! directory.
//!
//! Overwrite policy is per file and deliberately not uniform: the manifest, the
//! loader and the version pin are generated code and are always refreshed, the
//! binary is always replaced, and only `aphrodite.toml` is preserved unless
//! `--force`. Getting that backwards either freezes a stale loader against a new
//! dylib or silently discards the user's configuration.
//!
//! Registration is not here: it is an outcome rather than a file write, so it lives
//! in `Setup::Register`.

pub mod BinaryName;
pub mod BinaryVersion;
pub mod ConfigurationTemplate;
pub mod InitPy;
pub mod Permissions;
pub mod PluginShim;
pub mod PluginYAML;
