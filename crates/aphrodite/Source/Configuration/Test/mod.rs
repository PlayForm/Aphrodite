//! Tests for the configuration model, split by what is under test: the env
//! readers (`Environment`), the multi-proxy document (`Parse`) and the
//! precedence chain that turns one into a `CLI` (`Resolve`).
//!
//! `Guard` serializes the tests that mutate process-global environment
//! variables - `cargo test` runs a module's tests concurrently by default, and
//! every `test_resolve_*` case sets and clears the same `APHRODITE_*` vars.

mod Environment;
mod Guard;
mod Parse;
mod Resolve;

use crate::Configuration::Proxy::Multi::Struct::Struct as MultiConfiguration;

/// Parse a TOML literal into the multi-proxy model.
///
/// Shared by the parse and resolve areas: every one of those tests is written
/// as "here is the file, what does it resolve to", so the panic-on-invalid
/// wrapper belongs in one place.
pub(super) fn MultiConfigurationFromToml(TomlStr:&str) -> MultiConfiguration {
	toml::from_str(TomlStr).expect("valid test TOML")
}
