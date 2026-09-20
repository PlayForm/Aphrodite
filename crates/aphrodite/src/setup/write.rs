use std::{fs, io, path::Path, process::Command};

use super::*;
use crate::config::SetupArgs;

/// aphrodite.toml template - embedded at compile time.
/// Placeholders: `{api_url}`, `{model}`, `{cache_port}`, `{token_port}` -
/// replaced with user-provided values.
pub(crate) const CONFIG_TEMPLATE:&str = include_str!("../../templates/aphrodite.toml");

/// Write plugin.yaml manifest.
///
/// Always overwritten (03-F8): the manifest is generated code whose tool/hook
/// list and `version:` must track the installed binary, so a re-run of
/// `aphrodite setup` (e.g. after `cargo install aphrodite@<newer>`) must
/// refresh it rather than freeze it at the first-install version. Only the
/// user-editable `aphrodite.toml` stays `--force`-gated.
pub(crate) fn write_plugin_yaml(ctx:&SetupCtx, args:&SetupArgs) -> Result<(), SetupError> {
	let path = ctx.aphrodite_dir.join("plugin.yaml");

	let yaml = format!(
		r#"name: aphrodite
version: {version}
description: "CCR compression plugin - 13 tools, context engine, TOML-driven templates."
kind: standalone
min_hermes_version: "0.16.0"
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
  Proxies: token (:{token_port}, SQLite), cache (:{cache_port}, in-memory).
"#,
		version = env!("CARGO_PKG_VERSION"),
		token_port = args.token_port,
		cache_port = args.cache_port,
	);
	println!("writing plugin manifest -> {}", path.display());
	fs::write(&path, &yaml)?;
	secure_perms(&path, 0o644)?;
	Ok(())
}

/// Write __init__.py shim for hermes plugin loading.
///
/// Embeds `plugins/aphrodite/__init__.py` (the monorepo's live Hermes
/// plugin) directly, rather than maintaining a separate hand-copied template
/// (report 07 F14/T8) - a hand-maintained second copy had drifted
/// significantly: stale dylib-reload/free_string handling (report 06 F1),
/// a `register_tool` call with the wrong argument count, no skills
/// registration, no version handshake, no port-env reads, no health poll,
/// and stderr piped to `DEVNULL` (silently re-introducing a startup-failure
/// bug the live plugin had already fixed).
///
/// `templates/__init__.py` is a *copy* of the live plugin (the crate can't
/// `include_str!` a path outside its own package and still `cargo package`),
/// so the two are kept in sync by the
/// `test_hermes_plugin_shim_template_matches_live` drift guard below (03-F10),
/// which fails if they diverge.
pub(crate) const HERMES_PLUGIN_SHIM:&str = include_str!("../../templates/__init__.py");

pub(crate) fn write_init_py(ctx:&SetupCtx) -> Result<(), SetupError> {
	let path = ctx.aphrodite_dir.join("__init__.py");

	// Always overwritten (03-F8): the shim is code, not config - its FFI symbol
	// list and registration logic must match the freshly-installed dylib, so a
	// re-run of `aphrodite setup` must refresh a doctored/stale shim rather than
	// preserve it.
	println!("writing __init__.py -> {}", path.display());
	fs::write(&path, HERMES_PLUGIN_SHIM)?;
	secure_perms(&path, 0o644)?;
	Ok(())
}

/// Register plugin with hermes.
pub(crate) fn register_plugin(_ctx:&SetupCtx) -> Result<(), SetupError> {
	let status = Command::new("hermes")
		.args(["plugins", "enable", "aphrodite"])
		.output()
		.map_err(|e| SetupError::PluginRegistrationFailed(format!("hermes plugins enable: {e}")))?;

	if !status.status.success() {
		let stderr = String::from_utf8_lossy(&status.stderr);
		eprintln!("warning: hermes plugins enable aphrodite: {stderr}");
	} else {
		println!("plugin registered with hermes");
	}
	Ok(())
}

/// Set strict file permissions (Unix only).
pub(crate) fn secure_perms(path:&Path, mode:u32) -> io::Result<()> {
	#[cfg(unix)]
	{
		use std::os::unix::fs::PermissionsExt;
		let mut perms = fs::metadata(path)?.permissions();
		perms.set_mode(mode);
		fs::set_permissions(path, perms)?;
	}
	#[cfg(not(unix))]
	let _ = (path, mode);
	Ok(())
}

pub(crate) fn binary_name() -> &'static str { if cfg!(target_os = "windows") { "aphrodite.exe" } else { "aphrodite" } }
