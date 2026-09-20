//! Build-time FFI codegen: cbindgen → aphrodite_hermes.h → ctypesgen →
//! plugins/aphrodite/_bindings.py (the committed artifact the plugin imports).
//!
//! Graceful-degradation contract (see CODEGEN.md):
//!   - `APHRODITE_SKIP_FFI_GEN=1`           → cargo:warning + skip (escape hatch)
//!   - cbindgen generation fails             → cargo:warning + skip
//!   - ctypesgen tool not found              → cargo:warning + skip
//!   - ctypesgen run / post-process fails    → cargo:warning + skip
//!   - ctypesgen variant is NOT upstream
//!     (pypdfium2 fork or unrecognized)      → cargo:warning (finalize re-emits
//!     `CTYPESGEN_FORK_WARNING:` when the raw output shape is not the upstream
//!     loop form and skips - the fork's flat declaration form is not rewritable)
//!   - CONTRACT VIOLATION (a pointer-returning export declared with a
//!     non-pointer-width restype, or an export missing from the bindings)
//!     when the tools WERE available        → PANIC (fail the build loudly)
//!
//! In every skip path the committed plugins/aphrodite/_bindings.py remains in
//! effect; the plugin additionally falls back to its manual restype setup
//! when the artifact is absent entirely (fresh checkout, standalone
//! `aphrodite setup` installs whose embedded template ships without it).
//!
//! Header strategy (LEAN-UP #2/#9): OUT_DIR/aphrodite_hermes.h doubles as the
//! copy-on-change marker. ctypesgen 2.7.4 cannot read the header from stdin -
//! `-` is treated as a literal filename and fails inside gcc -E (verified on
//! this machine) - so the header stays a real file in OUT_DIR, cargo's
//! designated build-artifact dir. When cbindgen output is byte-identical to
//! the previous generation, the ctypesgen + finalize chain is skipped
//! entirely: a lib.rs edit that does not move the ABI costs nothing.

use std::{env, path::PathBuf, process::Command};

/// Every pointer-returning export of the crate. Must match src/lib.rs and
/// plugins/aphrodite/__init__.py::_REQUIRED_VOID_P. `codegen/finalize_bindings.py`
/// verifies the generated bindings declare ALL of these with a pointer-width
/// restype (c_void_p) - a c_int declaration would truncate the 64-bit pointer
/// and re-open the historical SIGSEGV bug class.
const POINTER_RETURNING_EXPORTS:&[&str] = &[
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

fn cargo_warning(msg:&str) {
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

	// Shared degraded-path exit (LEAN-UP #1): every early return below ends
	// with the committed artifact still in effect.
	let skip = |msg:&str| {
		cargo_warning(&format!(
			"{msg}; skipping FFI generation - the committed plugins/aphrodite/_bindings.py remains in effect"
		));
	};

	// ── Force-skip escape hatch (offline/dev-loop machines) ──
	if matches!(env::var("APHRODITE_SKIP_FFI_GEN").as_deref(), Ok("1") | Ok("true")) {
		skip("APHRODITE_SKIP_FFI_GEN=1: cbindgen+ctypesgen FFI generation disabled");
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
			skip(&format!("cbindgen.toml could not be loaded ({e})"));
			return;
		},
	};
	let bindings = match cbindgen::Builder::new().with_crate(&crate_dir).with_config(config).generate() {
		Ok(b) => b,
		Err(e) => {
			skip(&format!("cbindgen generation failed ({e})"));
			return;
		},
	};
	// cbindgen 0.29 dropped Display on Bindings - write to a Vec<u8> buffer.
	// The header is valid UTF-8 by construction (LEAN-UP #3): the bytes go
	// straight to the equality check and to disk with no lossy String
	// round-trip.
	let mut header_bytes = Vec::new();
	bindings.write(&mut header_bytes);

	let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
	// OUT_DIR/aphrodite_hermes.h doubles as the copy-on-change marker
	// (LEAN-UP #9): it persists across incremental builds of this
	// crate+profile, so it is both the previous-generation reference AND the
	// ctypesgen input.
	let header_path = out_dir.join("aphrodite_hermes.h");
	let raw_bindings_path = out_dir.join("_bindings.raw.py");

	// Copy-on-change: byte-identical header ⇒ the ABI did not move ⇒ the
	// committed _bindings.py is still exactly what this header produces, so
	// skip the ctypesgen + finalize chain entirely.
	if let Ok(prev) = std::fs::read(&header_path)
		&& prev == header_bytes
	{
		cargo_warning(
			"header unchanged - skipping ctypesgen/finalize (committed plugins/aphrodite/_bindings.py already matches \
			 this ABI)",
		);
		return;
	}

	if let Err(e) = std::fs::write(&header_path, &header_bytes) {
		skip(&format!("could not write {} ({e})", header_path.display()));
		return;
	}

	// ── ctypesgen availability probe ──
	// Prefer `python3 -m ctypesgen` (modern pip/venv installs); fall back to
	// the `ctypesgen` console script (homebrew/pip entry point). The probe
	// runs `--version` so a broken install degrades the same way, and the
	// version string classifies the variant: upstream ctypesgen reports a
	// semver-ish git-describe string; the pypdfium2-team fork (which emits a
	// flat `NAME = _libs[LIB][NAME]` form the upstream pattern set cannot
	// rewrite) identifies itself by name.
	let Some((ctypesgen_cmd, ctypesgen_version)) = probe_ctypesgen() else {
		skip("ctypesgen not found (tried `python3 -m ctypesgen` and `ctypesgen`)");
		return;
	};
	report_ctypesgen_variant(&ctypesgen_version);

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
			skip(&format!("ctypesgen exited {s}"));
			return;
		},
		Err(e) => {
			skip(&format!("ctypesgen could not run ({e})"));
			return;
		},
	}

	// ── Post-process + validate + copy-on-change ──
	// finalize_bindings.py exit codes: 0 = ok (installed or unchanged);
	// 1 = FFI CONTRACT VIOLATION (tools were available - the committed
	// artifact must not silently encode a truncating declaration, so the
	// build FAILS); 2+ = tool/script failure (warn + skip).
	// LEAN-UP #8: finalize_bindings.py STILL accepts `--required` (the
	// partner-side change that derives pointer-returning exports from the
	// header has not landed), so build.rs keeps the current call shape -
	// build.rs and finalize must not disagree mid-flight. Drop `--required`
	// in lockstep with that partner change.
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
	match &output {
		Ok(o) if o.status.success() => {
			// Generated bindings are in place (copy-on-change applied).
		},
		Ok(o) => {
			let stderr = String::from_utf8_lossy(&o.stderr);
			let code = o.status.code().unwrap_or(-1);
			if code == 1 {
				panic!("FFI CONTRACT VIOLATION in generated bindings (see finalize_bindings.py output):\n{stderr}");
			}
			skip(&format!("finalize_bindings.py exited {code}: {stderr}"));
		},
		Err(e) => {
			skip(&format!("finalize_bindings.py could not run ({e})"));
		},
	}
	// finalize_bindings.py reports non-upstream ctypesgen output shapes
	// (pypdfium2 fork / unknown) through a CTYPESGEN_FORK_WARNING: marker -
	// surface it as a cargo:warning so the fork is identifiable in build logs.
	if let Ok(o) = &output {
		let combined = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
		for line in combined.lines() {
			if let Some(rest) = line.strip_prefix("CTYPESGEN_FORK_WARNING: ") {
				cargo_warning(rest);
			}
		}
	}
}

fn probe_ctypesgen() -> Option<(Vec<String>, String)> {
	for candidate in [
		vec!["python3".to_string(), "-m".to_string(), "ctypesgen".to_string()],
		vec!["ctypesgen".to_string()],
	] {
		let probe = Command::new(&candidate[0]).args(&candidate[1..]).arg("--version").output();
		if let Ok(out) = probe
			&& out.status.success()
		{
			let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
			return Some((candidate, version));
		}
	}
	None
}

/// Warn (cargo:warning) when the ctypesgen variant is NOT upstream
/// ctypesgen - the upstream case stays silent. Upstream reports a semver-ish
/// git-describe version (e.g. 2.7.4-27202-gb3625f73d3); the pypdfium2-team
/// fork identifies itself by name; anything else is unrecognized and the
/// upstream pattern set may not apply.
fn report_ctypesgen_variant(version:&str) {
	let v = version.trim();
	if v.is_empty() {
		cargo_warning(
			"ctypesgen --version reported nothing - cannot classify the ctypesgen variant; the upstream pattern set \
			 in finalize_bindings.py may not apply (an unrecognized output shape warns and skips, keeping the \
			 committed plugins/aphrodite/_bindings.py in effect)",
		);
		return;
	}
	let lower = v.to_lowercase();
	if lower.contains("pypdfium2") || lower.contains("pdfium") {
		cargo_warning(&format!(
			"ctypesgen variant: pypdfium2-team fork (version {v}) - it emits a flat `NAME = _libs[LIB][NAME]` \
			 declaration form with single-line restype, NOT the upstream `for _lib in _libs.values()` loops; \
			 finalize_bindings.py will warn (CTYPESGEN_FORK_WARNING) and skip, keeping the committed \
			 plugins/aphrodite/_bindings.py in effect"
		));
	} else if !v.chars().next().is_some_and(|c| c.is_ascii_digit()) {
		cargo_warning(&format!(
			"ctypesgen variant: unrecognized version string ({v:?}) - the upstream pattern set in \
			 finalize_bindings.py may not apply; an unrecognized output shape warns and skips, keeping the committed \
			 plugins/aphrodite/_bindings.py in effect"
		));
	}
}
