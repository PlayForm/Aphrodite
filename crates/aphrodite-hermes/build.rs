//! Build-time FFI codegen: cbindgen → aphrodite_hermes.h → ctypesgen →
//! plugins/aphrodite/_bindings.py (the committed artifact the plugin imports).
//!
//! Graceful-degradation contract (see CODEGEN.md):
//!   - `APHRODITE_SKIP_FFI_GEN=1`           → cargo:warning + skip (escape hatch)
//!   - cbindgen generation fails             → cargo:warning + skip
//!   - ctypesgen tool not found              → cargo:warning + skip
//!   - ctypesgen run / post-process fails    → cargo:warning + skip
//!   - CONTRACT VIOLATION (a pointer-returning export declared with a
//!     non-pointer-width restype, or an export missing from the bindings)
//!     when the tools WERE available        → PANIC (fail the build loudly)
//!
//! In every skip path the committed plugins/aphrodite/_bindings.py remains in
//! effect; the plugin additionally falls back to its manual restype setup
//! when the artifact is absent entirely (fresh checkout, standalone
//! `aphrodite setup` installs whose embedded template ships without it).

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Every pointer-returning export of the crate. Must match src/lib.rs and
/// plugins/aphrodite/__init__.py::_REQUIRED_VOID_P. `codegen/finalize_bindings.py`
/// verifies the generated bindings declare ALL of these with a pointer-width
/// restype (c_void_p) - a c_int declaration would truncate the 64-bit pointer
/// and re-open the historical SIGSEGV bug class.
const POINTER_RETURNING_EXPORTS: &[&str] = &[
	"aphrodite_hermes_dispatch_tool",
	"aphrodite_hermes_list_tools",
	"aphrodite_hermes_get_schema",
	"aphrodite_hermes_version",
	"aphrodite_hermes_call_hook",
	"aphrodite_hermes_get_schemas",
	"aphrodite_hermes_get_hooks",
	"aphrodite_hermes_proxy_health",
	"aphrodite_hermes_materialize_directives",
];

fn cargo_warning(msg: &str) {
	println!("cargo:warning={msg}");
}

fn main() {
	let crate_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
	let plugin_dir = crate_dir.join("..").join("..").join("plugins").join("aphrodite");
	let plugin_init = plugin_dir.join("__init__.py");
	let plugin_bindings = plugin_dir.join("_bindings.py");
	let finalize_script = crate_dir.join("codegen").join("finalize_bindings.py");

	// Rebuild when any input to the generated bindings moves.
	println!("cargo:rerun-if-changed=src/lib.rs");
	println!("cargo:rerun-if-changed=cbindgen.toml");
	println!("cargo:rerun-if-changed={}", finalize_script.display());
	if plugin_init.exists() {
		println!("cargo:rerun-if-changed={}", plugin_init.display());
	}

	// ── Force-skip escape hatch (offline/dev-loop machines) ──
	if matches!(env::var("APHRODITE_SKIP_FFI_GEN").as_deref(), Ok("1") | Ok("true")) {
		cargo_warning(
			"APHRODITE_SKIP_FFI_GEN=1: skipping cbindgen+ctypesgen FFI generation; the committed plugins/aphrodite/_bindings.py remains in effect",
		);
		return;
	}

	// ── cbindgen ──
	// cbindgen is a build-dependency: by the time this script compiles, cargo
	// has already fetched/built it, so "availability" is guaranteed here. A
	// config/generation failure is treated as tool-unavailable and skipped -
	// never a build failure.
	let config = match cbindgen::Config::from_file(crate_dir.join("cbindgen.toml")) {
		Ok(c) => c,
		Err(e) => {
			cargo_warning(&format!(
				"cbindgen.toml could not be loaded ({e}); skipping FFI generation - the committed plugins/aphrodite/_bindings.py remains in effect"
			));
			return;
		},
	};
	let bindings = match cbindgen::Builder::new().with_crate(&crate_dir).with_config(config).generate() {
		Ok(b) => b,
		Err(e) => {
			cargo_warning(&format!(
				"cbindgen generation failed ({e}); skipping FFI generation - the committed plugins/aphrodite/_bindings.py remains in effect"
			));
			return;
		},
	};
	// cbindgen 0.29 dropped Display on Bindings - write to a Vec<u8> buffer.
	let mut header_bytes = Vec::new();
	bindings.write(&mut header_bytes);
	let header = String::from_utf8_lossy(&header_bytes).into_owned();

	// ── ctypesgen availability probe ──
	// Prefer `python3 -m ctypesgen` (modern pip/venv installs); fall back to
	// the `ctypesgen` console script (homebrew/pip entry point). The probe
	// runs `--version` so a broken install degrades the same way.
	let Some(ctypesgen_cmd) = probe_ctypesgen() else {
		cargo_warning(
			"ctypesgen not found (tried `python3 -m ctypesgen` and `ctypesgen`); skipping FFI generation - the committed plugins/aphrodite/_bindings.py remains in effect",
		);
		return;
	};

	let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
	let header_path = out_dir.join("aphrodite_hermes.h");
	let raw_bindings_path = out_dir.join("_bindings.raw.py");
	if let Err(e) = std::fs::write(&header_path, &header) {
		cargo_warning(&format!(
			"could not write {} ({e}); skipping FFI generation - the committed plugins/aphrodite/_bindings.py remains in effect",
			header_path.display()
		));
		return;
	}

	// ── ctypesgen: header → raw bindings module ──
	// `-l __APHRODITE_DYLIB__` is a placeholder only - finalize_bindings.py
	// neutralizes the import-time library load (the plugin's hot-reload
	// machinery owns the real CDLL handle).
	let mut cmd = Command::new(&ctypesgen_cmd[0]);
	cmd.args(&ctypesgen_cmd[1..]);
	let status = cmd
		.arg("-l")
		.arg("__APHRODITE_DYLIB__")
		.arg("-o")
		.arg(&raw_bindings_path)
		.arg(&header_path)
		.status();
	match status {
		Ok(s) if s.success() => {},
		Ok(s) => {
			cargo_warning(&format!(
				"ctypesgen exited {s}; skipping FFI generation - the committed plugins/aphrodite/_bindings.py remains in effect"
			));
			return;
		},
		Err(e) => {
			cargo_warning(&format!(
				"ctypesgen could not run ({e}); skipping FFI generation - the committed plugins/aphrodite/_bindings.py remains in effect"
			));
			return;
		},
	}

	// ── Post-process + validate + copy-on-change ──
	// finalize_bindings.py exit codes: 0 = ok (installed or unchanged);
	// 1 = FFI CONTRACT VIOLATION (tools were available - the committed
	// artifact must not silently encode a truncating declaration, so the
	// build FAILS); 2+ = tool/script failure (warn + skip).
	let output = Command::new("python3")
		.arg(&finalize_script)
		.arg("--header")
		.arg(&header_path)
		.arg("--raw-bindings")
		.arg(&raw_bindings_path)
		.arg("--output")
		.arg(&plugin_bindings)
		.arg("--required")
		.arg(POINTER_RETURNING_EXPORTS.join(","))
		.output();
	match output {
		Ok(o) if o.status.success() => {
			// Generated bindings are in place (copy-on-change applied).
		},
		Ok(o) => {
			let stderr = String::from_utf8_lossy(&o.stderr);
			let code = o.status.code().unwrap_or(-1);
			if code == 1 {
				panic!("FFI CONTRACT VIOLATION in generated bindings (see finalize_bindings.py output):\n{stderr}");
			}
			cargo_warning(&format!(
				"finalize_bindings.py exited {code}; skipping FFI generation - the committed plugins/aphrodite/_bindings.py remains in effect\n{stderr}"
			));
		},
		Err(e) => {
			cargo_warning(&format!(
				"finalize_bindings.py could not run ({e}); skipping FFI generation - the committed plugins/aphrodite/_bindings.py remains in effect"
			));
		},
	}
}

fn probe_ctypesgen() -> Option<Vec<String>> {
	for candidate in [
		vec!["python3".to_string(), "-m".to_string(), "ctypesgen".to_string()],
		vec!["ctypesgen".to_string()],
	] {
		let probe = Command::new(&candidate[0]).args(&candidate[1..]).arg("--version").output();
		if let Ok(out) = probe {
			if out.status.success() {
				return Some(candidate);
			}
		}
	}
	None
}
