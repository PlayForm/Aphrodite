use std::{fs, path::Path, process::Command};

use super::*;

// ── T8 (F14): the setup-embedded shim is now `include_str!`'d directly
// from the live plugin, so it can't drift - these pin the specific bugs
// the old hand-copied `templates/__init__.py` had regrown. ──
#[test]
fn test_hermes_plugin_shim_has_no_stderr_devnull() {
	assert!(
		!HERMES_PLUGIN_SHIM.contains("stderr=subprocess.DEVNULL"),
		"stderr must go to a log file, not DEVNULL (re-introduces the v1.2.1 silent-startup bug)"
	);
}

#[test]
fn test_hermes_plugin_shim_reads_no_auto_launch() {
	assert!(
		HERMES_PLUGIN_SHIM.contains(r#"os.environ.get("APHRODITE_NO_AUTO_LAUNCH""#),
		"the guard must be read, not just set"
	);
}

#[test]
fn test_hermes_plugin_shim_reads_port_env_vars() {
	assert!(HERMES_PLUGIN_SHIM.contains("APHRODITE_CACHE_PORT"));
	assert!(HERMES_PLUGIN_SHIM.contains("APHRODITE_TOKEN_PORT"));
}

#[test]
fn test_hermes_plugin_shim_gates_context_engine_opt_in() {
	assert!(HERMES_PLUGIN_SHIM.contains("APHRODITE_CONTEXT_ENGINE"));
}

#[test]
fn test_hermes_plugin_shim_registers_tools_with_toolset_arg() {
	// The old template called `ctx.register_tool(schema, handler)` (2
	// args) while the real Hermes API + live plugin use
	// `register_tool(name, toolset, schema, handler)` (4 args).
	assert!(HERMES_PLUGIN_SHIM.contains(r#"ctx.register_tool(name, "aphrodite", schema, "#));
}

// ── T8 (F10): drift guard. The embedded `templates/__init__.py` is a copy
// of the live `plugins/aphrodite/__init__.py`; nothing but this test keeps
// them from diverging (as they had before v1-04-F2). Compare CRLF-normalized
// so a working tree with `core.autocrlf=true` doesn't produce a false fail,
// and early-return when the live file is absent so a published-crate build
// (no submodule checkout) still passes. ──
#[test]
fn test_hermes_plugin_shim_template_matches_live() {
	let live_path = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("..")
		.join("..")
		.join("plugins")
		.join("aphrodite")
		.join("__init__.py");
	let Ok(live) = fs::read_to_string(&live_path) else {
		// Live plugin submodule not checked out (e.g. published-crate build)
		// - nothing to compare against, so this guard is a no-op.
		return;
	};
	let normalize = |s:&str| s.replace("\r\n", "\n");
	assert_eq!(
		normalize(&live),
		normalize(HERMES_PLUGIN_SHIM),
		"templates/__init__.py has drifted from the live plugins/aphrodite/__init__.py - re-copy the live plugin into \
		 crates/aphrodite/templates/__init__.py to keep the setup-embedded shim in sync"
	);
}

// Network-touching (real GitHub release) - not run by default, only on
// demand (`cargo test -p aphrodite --lib -- --ignored`) since the
// hermetic suite must stay network-free.
#[test]
#[ignore = "hits the real GitHub release - run explicitly to verify"]
fn test_verify_download_checksum_against_real_release() {
	let dir = std::env::temp_dir().join("aphrodite-checksum-test");
	fs::create_dir_all(&dir).unwrap();
	let release_dir = "https://github.com/PlayForm/Aphrodite/releases/download/Aphrodite/v1.3.2";
	let triple = "aarch64-apple-darwin";
	let asset = "aphrodite-aarch64-apple-darwin";
	let dest = dir.join(asset);
	let status = Command::new("curl")
		.args(["-fsSL", "-o", dest.to_str().unwrap(), &format!("{release_dir}/{asset}")])
		.status()
		.unwrap();
	assert!(status.success());

	// Correct hash passes.
	verify_download_checksum(release_dir, triple, asset, &dest).expect("real asset must verify clean");

	// A deliberately corrupted file must be rejected, not silently accepted.
	fs::write(&dest, b"corrupted content").unwrap();
	let result = verify_download_checksum(release_dir, triple, asset, &dest);
	assert!(result.is_err(), "corrupted asset must fail checksum verification");

	fs::remove_dir_all(&dir).unwrap();
}
