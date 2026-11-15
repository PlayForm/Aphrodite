//! Deciding which configuration file to read.
//!
//! Three candidates, in order: an explicit `APHRODITE_CONFIG_PATH`, a
//! `./aphrodite.toml` in the current directory, and the runtime home's
//! `aphrodite.toml`. The third exists because `aphrodite setup` writes its
//! generated config into the runtime home, and a setup run from a directory
//! without its own `aphrodite.toml` used to fall straight through to CLI-parse
//! mode, where the templated ports, model and api_url were never read.

use aphrodite::Home;

/// Resolve the config path and whether a config file was found there.
///
/// Returns `(path, use_multi_config)`. `UseMultiConfiguration` is `false` only when
/// no explicit `APHRODITE_CONFIG_PATH` was set and neither `./aphrodite.toml`
/// nor the runtime home's copy exists - the caller then parses the CLI instead.
///
/// An explicit `APHRODITE_CONFIG_PATH` is never redirected: an override that
/// does not exist surfaces as CLI fallback (or a clear error), never as a silent
/// switch to a different file, because an operator who named a path needs to
/// hear that the path is wrong.
pub fn Resolve() -> (String, bool) {
	let ExplicitConfigurationPath = std::env::var("APHRODITE_CONFIG_PATH").ok();
	let ConfigurationPath = ExplicitConfigurationPath
		.clone()
		.unwrap_or_else(|| "aphrodite.toml".to_string());
	let UseMultiConfiguration = std::path::Path::new(&ConfigurationPath).exists();
	match UseMultiConfiguration || ExplicitConfigurationPath.is_some() {
		true => (ConfigurationPath, UseMultiConfiguration),
		false => {
			// Runtime home resolution is a single shared decision
			// (Home::Runtime::Fn::Fn: $APHRODITE_HOME -> $HERMES_HOME -> $HOME ->
			// platform default) so a setup run under a non-default Hermes home
			// (Docker, profile gateways) finds the config the plugin shim will
			// use - never a second, shadow home (issue 40).
			let HomeConfiguration = Home::Configuration::Fn::Fn();
			match HomeConfiguration.exists() {
				true => (HomeConfiguration.to_string_lossy().into_owned(), true),
				false => (ConfigurationPath, false),
			}
		},
	}
}
