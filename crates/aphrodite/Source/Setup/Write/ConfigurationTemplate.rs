//! The embedded `aphrodite.toml` template.
//!
//! Embedded rather than read at run time so a cargo-installed binary carries its
//! own install payload and has no dependency on a source checkout or a data
//! directory. The file is `crates/aphrodite/templates/aphrodite.toml`.

#![allow(non_upper_case_globals)]

/// The `aphrodite.toml` template, compiled in.
///
/// Placeholders `{api_url}`, `{model}`, `{cache_port}` and `{token_port}` are
/// substituted with the user's values by the setup entry point; every other byte
/// is written through unchanged.
///
/// It is not a schema and it is not validated here: a template that stops parsing
/// as TOML still installs, and the failure appears when the binary loads its
/// configuration. A placeholder that no longer matches what setup replaces is
/// written out literally, so the two have to be edited together.
pub(crate) const Const:&str = include_str!("../../../templates/aphrodite.toml");
