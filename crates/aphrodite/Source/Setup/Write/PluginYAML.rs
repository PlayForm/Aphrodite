//! The generated Hermes `plugin.yaml` manifest.
//!
//! The manifest is generated code, not configuration: its tool and hook lists and
//! its `version:` must track the installed binary, so setup rewrites it on every
//! run. It lands in the hooks-only plugin directory next to the `__init__.py`
//! loader; everything else setup installs lives in the runtime home.

use std::fs;

use anyhow::Context;

use crate::{
	Configuration::CLI::Setup::Struct::Struct as SetupArgs,
	Setup::{Run::Context::Struct as SetupContext, Write::Permissions::Fn as SecurePerms},
};

/// Write `plugin.yaml` into the plugin directory.
///
/// Always overwritten (03-F8): a re-run of `aphrodite setup`, for example after
/// `cargo install aphrodite@<newer>`, must refresh the manifest rather than freeze
/// it at the first-install version. The version, the two proxy ports and the
/// install message are interpolated from the binary and the CLI arguments.
///
/// It is not user-editable and it is not `--force`-gated - only `aphrodite.toml`
/// is. Leaving a stale manifest behind means Hermes advertises tools the installed
/// dylib does not export, and the plugin fails at registration with a missing-tool
/// error instead of at install with a version mismatch.
pub(crate) fn Fn(Context:&SetupContext, Args:&SetupArgs) -> anyhow::Result<()> {
	let path = Context.PluginDirectory.join("plugin.yaml");
	fs::create_dir_all(&Context.PluginDirectory)
		.with_context(|| format!("create the plugin directory {}", Context.PluginDirectory.display()))?;

	let YAML = format!(
		r#"name: aphrodite
version: {version}
description: "CCR compression plugin - 28-type classifier, 13 tools, opt-in context engine, TOML-driven templates. Disclosure - pre_tool_call auto-backgrounds long terminal/process commands (modify: background=true, notify_on_complete=true) and rewrites chained terminal commands when enabled."
kind: standalone
requires_hermes: ">=0.20.2"
requires_hooks: true
provides_hooks:
  - on_session_start
  - transform_tool_result
  - pre_llm_call
  - transform_terminal_output
  - post_llm_call
provides_tools:
  - aphrodite_retrieve
  - aphrodite_compress
  - aphrodite_stats
  - aphrodite_rebuild
  - aphrodite_files
  - aphrodite_diff
  - aphrodite_search
  - aphrodite_directive
  - aphrodite_test
  - aphrodite_catalog
  - aphrodite_reclassify
  - aphrodite_prefetch
  - aphrodite_prefetch_status
provides_context_engine: true
install_message: |
  aphrodite v{version} - installed via `cargo install aphrodite` + `aphrodite setup`.
  All logic in binaries/ - Rust-powered. Secure defaults.
  Proxies: token (:{TokenPort}, SQLite), cache (:{CachePort}, in-memory).
"#,
		version = env!("CARGO_PKG_VERSION"),
		TokenPort = Args.TokenPort,
		CachePort = Args.CachePort,
	);
	println!("writing plugin manifest -> {}", path.display());
	fs::write(&path, &YAML).with_context(|| format!("write the plugin manifest {}", path.display()))?;
	SecurePerms(&path, 0o644)?;
	Ok(())
}
