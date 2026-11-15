//! Tests pinning the embedded Hermes plugin loader.
//!
//! The setup-embedded shim is `include_str!`'d from the live plugin, so it cannot
//! drift by editing; these pin the specific bugs an older hand-copied
//! `templates/__init__.py` had regrown, plus the guard that the embedded copy still
//! matches `plugins/aphrodite/__init__.py` byte for byte.

use std::{fs, path::Path};

use crate::Setup::Write::PluginShim::Const as HermesPluginShim;

// ── T8 (F14): the setup-embedded shim is now `include_str!`'d directly
// from the live plugin, so it can't drift - these pin the specific bugs
// the old hand-copied `templates/__init__.py` had regrown. ──
#[test]
fn TestHermesPluginShimHasNoStderrDevnull() {
	assert!(
		!HermesPluginShim.contains("stderr=subprocess.DEVNULL"),
		"stderr must go to a log file, not DEVNULL (re-introduces the v1.2.1 silent-startup bug)"
	);
}

#[test]
fn TestHermesPluginShimReadsNoAutoLaunch() {
	assert!(
		HermesPluginShim.contains(r#"os.environ.get("APHRODITE_NO_AUTO_LAUNCH""#),
		"the guard must be read, not just set"
	);
}

#[test]
fn TestHermesPluginShimReadsPortEnvironmentVars() {
	assert!(HermesPluginShim.contains("APHRODITE_CACHE_PORT"));
	assert!(HermesPluginShim.contains("APHRODITE_TOKEN_PORT"));
}

#[test]
fn TestHermesPluginShimGatesContextEngineOptIn() {
	assert!(HermesPluginShim.contains("APHRODITE_CONTEXT_ENGINE"));
}

#[test]
fn TestHermesPluginShimRegistersToolsWithToolsetArgument() {
	// The old template called `ctx.register_tool(schema, handler)` (2
	// args) while the real Hermes API + live plugin use
	// `register_tool(name, toolset, schema, handler)` (4 args).
	assert!(HermesPluginShim.contains(r#"ctx.register_tool(name, "aphrodite", schema, "#));
}

// ── T8 (F10): drift guard. The embedded `templates/__init__.py` is a copy
// of the live `plugins/aphrodite/__init__.py`; nothing but this test keeps
// them from diverging (as they had before v1-04-F2). Compare CRLF-normalized
// so a working tree with `core.autocrlf=true` doesn't produce a false fail,
// and early-return when the live file is absent so a published-crate build
// (no submodule checkout) still passes. ──
#[test]
fn TestHermesPluginShimTemplateMatchesLive() {
	let LivePath = Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("..")
		.join("..")
		.join("plugins")
		.join("aphrodite")
		.join("__init__.py");
	let Ok(Live) = fs::read_to_string(&LivePath) else {
		// Live plugin submodule not checked out (e.g. published-crate build)
		// - nothing to compare against, so this guard is a no-op.
		return;
	};
	let Normalize = |s:&str| s.replace("\r\n", "\n");
	assert_eq!(
		Normalize(&Live),
		Normalize(HermesPluginShim),
		"templates/__init__.py has drifted from the live plugins/aphrodite/__init__.py - re-copy the live plugin into \
		 crates/aphrodite/templates/__init__.py to keep the setup-embedded shim in sync"
	);
}
