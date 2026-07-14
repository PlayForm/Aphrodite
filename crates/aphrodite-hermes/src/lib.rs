//! aphrodite-hermes: Hermes Agent-specific integration crate.
//!
//! This crate handles all Hermes-specific concerns - tool schemas,
//! hook dispatch, skill registration - leaving the core `aphrodite`
//! crate as a pure, agent-agnostic compression engine.
//!
//! Architecture:
//!   Python plugin (thin loader) → ctypes → libaphrodite_hermes.dylib
//!                                           ├─ Tool dispatch (compress, retrieve, stats, etc.)
//!                                           ├─ Hook dispatch (on_session_start, transform, terminal, pre/post LLM)
//!                                           └─ Skill registration
//!                                           ↓ (depends on)
//!                                    aphrodite crate (rlib)
//!                                           ├─ Core compression (hooks, state, marker)
//!                                           ├─ Resolution (resolve, stage2, struct)
//!                                           └─ Catalog, session, prefetch, config

// See crates/aphrodite/src/lib.rs's matching comment: fixing this properly
// (marking every `pub extern "C" fn` `unsafe`) is report 03's job, not a
// side effect of wiring up a CI clippy gate.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

mod schemas;
mod skills;
mod tools;

use std::{
	ffi::{CStr, CString},
	os::raw::c_char,
	sync::{Mutex, OnceLock},
};

// Re-export core aphrodite types for convenience
pub use aphrodite::state::AphroditeState;

// ── Process-global session state ───────────────────────────
//
// The Hermes plugin loads this dylib once per process and drives a single
// agent session, so one shared `AphroditeState` is the correct model (it
// mirrors the proxy's per-session store and the core crate's handle map).
// Every hook and tool call operates on this shared state, so compressions
// stored by `transform_tool_result` survive long enough for
// `aphrodite_retrieve` to resolve them. The lock is poison-tolerant: a panic in
// one call must not wedge every later call.

/// Access the process-global session state.
///
/// Initializes from `aphrodite.toml` (report 07 F3/T16) via
/// `config_loader::Config` - previously this crate always ran on hardcoded
/// `AphroditeState::default()` values; `Config::load()`/`apply_compression`
/// were the only implementation of the advertised env>TOML>default
/// resolution for this path but had zero call sites anywhere in the repo.
///
/// Test builds skip the TOML load and use plain defaults: `Config::load()`
/// searches `./aphrodite.toml` first, and this crate's own tests run with
/// the workspace root (which HAS a real `aphrodite.toml`) as their working
/// directory - without this split, unit tests would non-hermetically pick
/// up whatever the repo's live config happens to contain instead of the
/// documented defaults they assert against.
pub(crate) fn shared() -> &'static Mutex<AphroditeState> {
	static STATE: OnceLock<Mutex<AphroditeState>> = OnceLock::new();
	STATE.get_or_init(|| {
		#[cfg(not(test))]
		let state = {
			let mut s = AphroditeState::default();
			aphrodite::config_loader::Config::load().apply_compression(&mut s);
			s
		};
		#[cfg(test)]
		let state = AphroditeState::default();
		Mutex::new(state)
	})
}

/// Run `f` against the shared session state under the global lock.
///
/// Torn-state contract (report 06 F12): `with_shared` itself does not
/// `catch_unwind` - the panic guard lives one layer up, in `guarded()` at
/// every call site that dispatches into this fn. If `f` panics partway
/// through a multi-step mutation, whatever partial state existed at the
/// panic point is what the next caller observes (the lock itself is fine -
/// poison is recovered via `into_inner` above). Keep multi-step mutations
/// (e.g. `retain` then `push_front` in `inline_store_put`) ordered so an
/// interruption degrades to "entry missing" rather than "duplicate entry".
pub(crate) fn with_shared<T>(f: impl FnOnce(&mut AphroditeState) -> T) -> T {
	let mut guard = shared().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	f(&mut guard)
}

/// Serializes tests that assert across multiple state-mutating calls, since
/// the shared session state is process-global and `on_session_start` resets it.
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
	static G: OnceLock<Mutex<()>> = OnceLock::new();
	G.get_or_init(|| Mutex::new(()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Map a hook compression result to the value Hermes uses to replace output.
///
/// Hermes only honors a *string* return from `transform_tool_result` /
/// `transform_terminal_output` (first non-None string wins; non-strings pass
/// through). So return the CCR marker string when compression happened, and
/// `null` otherwise to leave the original output untouched.
pub(crate) fn replacement_from(r: &serde_json::Value) -> serde_json::Value {
	if r.get("compressed").and_then(|v| v.as_bool()).unwrap_or(false) {
		if let Some(marker) = r.get("marker").and_then(|v| v.as_str()) {
			return serde_json::Value::String(marker.to_string());
		}
	}
	serde_json::Value::Null
}

/// Default cache proxy port, used when `APHRODITE_CACHE_PORT` is unset.
const DEFAULT_CACHE_PORT: u16 = 9797;
/// Default token proxy port, used when `APHRODITE_TOKEN_PORT` is unset.
const DEFAULT_TOKEN_PORT: u16 = 9798;

/// Resolve the cache/token proxy ports for this process.
///
/// Reads `APHRODITE_CACHE_PORT` / `APHRODITE_TOKEN_PORT` from the environment
/// so that multiple concurrent Hermes Agent instances on the same machine can
/// each be pointed at their own proxy pair (see `aphrodite setup --cache-port
/// / --token-port`), falling back to the historical 9797/9798 defaults.
fn configured_ports() -> (u16, u16) {
	// F11/F15: warn on a malformed (present but unparseable) value instead of
	// silently falling back - a missing var is unremarkable, but a typo'd one
	// left an operator with no way to tell "my override didn't apply" from "I
	// didn't set an override" (the exact bug class `apply_port_override`'s
	// comment in `aphrodite::config` documents).
	// This crate has no logging/tracing subscriber of its own (it's a dylib
	// loaded into the host Python process, not a standalone binary) - use
	// `eprintln!` directly so the warning actually reaches the host's
	// captured stderr instead of a silently-unsubscribed `tracing` call.
	let port_from_env = |var: &str, default: u16| match std::env::var(var) {
		Ok(v) => match v.parse::<u16>() {
			Ok(port) => port,
			Err(_) => {
				eprintln!(
					"aphrodite-hermes: {}={:?} is not a valid port (1-65535); using default {}",
					var, v, default
				);
				default
			},
		},
		Err(_) => default,
	};
	(
		port_from_env("APHRODITE_CACHE_PORT", DEFAULT_CACHE_PORT),
		port_from_env("APHRODITE_TOKEN_PORT", DEFAULT_TOKEN_PORT),
	)
}

/// Probe whether the cache and token proxies are listening.
pub(crate) fn proxy_health() -> serde_json::Value {
	use std::{net::TcpStream, time::Duration};
	let timeout = Duration::from_millis(400);
	let alive = |addr: String| {
		addr.parse()
			.ok()
			.and_then(|a| TcpStream::connect_timeout(&a, timeout).ok())
			.is_some()
	};
	let (cache_port, token_port) = configured_ports();
	serde_json::json!({
		"token": {"port": token_port, "alive": alive(format!("127.0.0.1:{token_port}"))},
		"cache": {"port": cache_port, "alive": alive(format!("127.0.0.1:{cache_port}"))},
	})
}

// ── C ABI helpers ──────────────────────────────────────────

/// Convert a caller-supplied C string pointer into an owned `String`, or an
/// empty string if `ptr` is null. Invalid UTF-8 is replaced lossily rather
/// than rejected, since a malformed argument should degrade gracefully, not
/// panic across the `extern "C"` boundary.
///
/// # Safety
/// `ptr`, if non-null, must point to a valid, NUL-terminated C string that
/// stays valid for the duration of this call (the standard `CStr::from_ptr`
/// contract). Every caller in this crate passes pointers received directly
/// from Hermes's C ABI call, which are expected to uphold that contract.
unsafe fn cstr_to_string(ptr: *const c_char) -> String {
	if ptr.is_null() {
		String::new()
	} else {
		CStr::from_ptr(ptr).to_string_lossy().into_owned()
	}
}

fn to_c_string(s: &str) -> *mut c_char {
	CString::new(s).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut())
}

fn to_json_error(msg: &str) -> *mut c_char {
	to_c_string(&serde_json::json!({"error": msg}).to_string())
}

/// Run `f` under `catch_unwind`, converting a panic into an error-JSON string
/// instead of letting it unwind across the `extern "C"` boundary (which would
/// otherwise trigger the Rust runtime's forced process abort - this crate had
/// zero panic guards until this was added). Every exported fn except
/// `aphrodite_hermes_free_string` (must stay minimal/infallible) routes
/// through this.
fn guarded(f: impl FnOnce() -> *mut c_char + std::panic::UnwindSafe) -> *mut c_char {
	std::panic::catch_unwind(f).unwrap_or_else(|_| to_json_error("internal error: panicked in aphrodite-hermes"))
}

// ── Tool dispatch C ABI ────────────────────────────────────

/// Dispatch an aphrodite tool call by name.
/// Returns JSON result string. Caller must free with
/// aphrodite_hermes_free_string.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_dispatch_tool(tool_name: *const c_char, args_json: *const c_char) -> *mut c_char {
	let name = unsafe { cstr_to_string(tool_name) };
	let args = unsafe { cstr_to_string(args_json) };

	guarded(std::panic::AssertUnwindSafe(move || {
		let result = tools::dispatch(&name, &args);
		match serde_json::to_string(&result) {
			Ok(json) => to_c_string(&json),
			Err(e) => to_json_error(&format!("serialize error: {}", e)),
		}
	}))
}

/// List all registered Hermes tool schemas as JSON array.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_list_tools() -> *mut c_char {
	guarded(|| {
		let schemas = schemas::all_schemas();
		to_c_string(&serde_json::to_string(&schemas).unwrap_or_default())
	})
}

/// List all bundled skill names and descriptions as JSON.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_list_skills() -> *mut c_char {
	guarded(|| {
		let skills = skills::all_skills();
		to_c_string(&serde_json::to_string(&skills).unwrap_or_default())
	})
}

/// Get a single tool schema by name.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_get_schema(tool_name: *const c_char) -> *mut c_char {
	let name = unsafe { cstr_to_string(tool_name) };
	guarded(std::panic::AssertUnwindSafe(move || match schemas::get_schema(&name) {
		Some(s) => to_c_string(&serde_json::to_string(&s).unwrap_or_default()),
		None => to_json_error(&format!("unknown tool: {}", name)),
	}))
}

/// Free a string returned by any aphrodite_hermes_* function.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_free_string(s: *mut c_char) {
	if !s.is_null() {
		unsafe {
			let _ = CString::from_raw(s);
		}
	}
}

/// Version of this crate.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_version() -> *mut c_char {
	guarded(|| to_c_string(&serde_json::json!({"version": env!("CARGO_PKG_VERSION")}).to_string()))
}

// ── Hook dispatch C ABI ────────────────────────────────────

/// Call a Hermes hook by name with JSON args.
///
/// Operates on the process-global session state and honors the exact Hermes
/// hook contract (verified against the Hermes source):
///   - `transform_tool_result` - tool output arrives under `result`; return a
///     marker string to replace it, or `null` to pass through.
///   - `transform_terminal_output` - output arrives under `output`; same
///     return.
///   - `pre_llm_call` - return `{"context": "..."}` to inject a catalog
///     summary.
///   - `on_session_start` / `post_llm_call` - lifecycle; return value ignored.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_call_hook(hook_name: *const c_char, args_json: *const c_char) -> *mut c_char {
	let name = unsafe { cstr_to_string(hook_name) };
	let args = unsafe { cstr_to_string(args_json) };

	guarded(std::panic::AssertUnwindSafe(move || {
		// Parse args as JSON object
		let parsed: serde_json::Value = match serde_json::from_str(&args) {
			Ok(v) => v,
			Err(e) => return to_json_error(&format!("invalid args: {}", e)),
		};

		// Hermes passes tool output under `result` and terminal output under
		// `output` (not `content`). Accept `content` too for direct/test callers.
		let tool = parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
		let tool_content = parsed
			.get("result")
			.or_else(|| parsed.get("content"))
			.and_then(|v| v.as_str())
			.unwrap_or("");
		let term_content = parsed
			.get("output")
			.or_else(|| parsed.get("content"))
			.and_then(|v| v.as_str())
			.unwrap_or("");

		let result: serde_json::Value = with_shared(|state| {
			match name.as_str() {
				// Accept both the canonical Hermes name and the legacy alias.
				"on_session_start" | "session_start" => aphrodite::session::on_session_start(state),
				"transform_tool_result" => {
					// Hermes wraps every tool result in JSON (`{"output":...,
					// "exit_code":...}`, `{"total_count":...,"matches":[...]}`,
					// etc.) - unwrap it to classify/preview the real payload,
					// but hand core the ORIGINAL content so it's what gets
					// hashed and stored (retrieval must stay lossless).
					let classify = crate::tools::unwrap_hermes_result(tool_content);
					let r = aphrodite::hooks::transform_tool_result_classified(
						state,
						tool_content,
						tool,
						classify.as_ref().map(|(c, t)| (c.as_str(), t.as_str())),
					);
					replacement_from(&r)
				},
				"transform_terminal_output" => {
					let classify = crate::tools::unwrap_hermes_result(term_content);
					let r = aphrodite::hooks::transform_terminal_output_classified(
						state,
						term_content,
						classify.as_ref().map(|(c, t)| (c.as_str(), t.as_str())),
					);
					replacement_from(&r)
				},
				"pre_llm_call" => {
					// 01-F3: this is the ONLY pre_llm_call arm Hermes actually
					// calls in production - core's `hooks::pre_llm_call` (which
					// already builds directive context) has no caller on this
					// path, so the "wire directives into pre_llm_call" feature
					// shipped dead end-to-end. Append directive context to the
					// same `context` string Hermes already honors, rather than
					// adding a second field it wouldn't read.
					let summary = aphrodite::session::catalog_summary(state);
					let directives =
						aphrodite::directives::build_directive_context(&state.directives, &state.active_directives);
					let context = match (summary.is_empty(), directives.is_empty()) {
						(true, true) => String::new(),
						(false, true) => summary,
						(true, false) => directives,
						(false, false) => format!("{summary}\n{directives}"),
					};
					if context.is_empty() {
						serde_json::Value::Null
					} else {
						serde_json::json!({ "context": context })
					}
				},
				"post_llm_call" => {
					// Delegates to `hooks::post_llm_call`, not a bare `next_turn`
					// (report 06 F11/T13): this is the process-global dispatch
					// path the Hermes Python plugin actually calls on every
					// turn, so calling `next_turn` directly here bypassed the
					// turn-archive step entirely - `conv_index` stayed empty in
					// real usage even after wiring `hooks::post_llm_call`.
					aphrodite::hooks::post_llm_call(state);
					serde_json::Value::Null
				},
				_ => serde_json::json!({ "error": format!("unknown hook: {}", name) }),
			}
		});

		to_c_string(&serde_json::to_string(&result).unwrap_or_default())
	}))
}

/// Return all tool schemas as a JSON array.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_get_schemas() -> *mut c_char {
	guarded(|| {
		let schemas = schemas::all_schemas();
		to_c_string(&serde_json::json!(schemas).to_string())
	})
}

/// Return hook names as a JSON array.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_get_hooks() -> *mut c_char {
	// Hermes invokes the session hook as `on_session_start` (the `on_` prefix is
	// required by its VALID_HOOKS table); registering `session_start` silently
	// no-ops. The other four names match Hermes verbatim.
	guarded(|| {
		to_c_string(
			&serde_json::json!([
				"on_session_start",
				"transform_tool_result",
				"transform_terminal_output",
				"pre_llm_call",
				"post_llm_call"
			])
			.to_string(),
		)
	})
}

/// Probe proxy health via TCP connect.
#[no_mangle]
pub extern "C" fn aphrodite_hermes_proxy_health() -> *mut c_char {
	guarded(|| to_c_string(&proxy_health().to_string()))
}

#[cfg(test)]
mod tests {
	use std::ffi::CString;

	use super::*;

	/// Serializes tests that touch process-global env vars
	/// (`APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT`).
	fn env_guard() -> std::sync::MutexGuard<'static, ()> {
		static G: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
		G.get_or_init(|| std::sync::Mutex::new(()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner)
	}

	// ── T9 (F11/F15): a malformed port env var must fall back to the
	// default, not propagate a parse failure or silently pick something
	// else - and it must not panic. ──
	#[test]
	fn test_configured_ports_falls_back_on_malformed_value() {
		let _g = env_guard();
		std::env::set_var("APHRODITE_CACHE_PORT", "not-a-port");
		std::env::remove_var("APHRODITE_TOKEN_PORT");
		let (cache, token) = configured_ports();
		std::env::remove_var("APHRODITE_CACHE_PORT");
		assert_eq!(cache, DEFAULT_CACHE_PORT);
		assert_eq!(token, DEFAULT_TOKEN_PORT);
	}

	#[test]
	fn test_configured_ports_honors_valid_override() {
		let _g = env_guard();
		std::env::set_var("APHRODITE_CACHE_PORT", "19797");
		let (cache, _token) = configured_ports();
		std::env::remove_var("APHRODITE_CACHE_PORT");
		assert_eq!(cache, 19797);
	}

	#[test]
	fn test_version_is_semver() {
		let json_ptr = aphrodite_hermes_version();
		let json = unsafe { CStr::from_ptr(json_ptr) }.to_string_lossy().into_owned();
		let v: serde_json::Value = serde_json::from_str(&json).unwrap();
		let ver = v["version"].as_str().unwrap();
		assert!(
			ver.starts_with("0.") || ver.starts_with("1."),
			"expected semver starting with 0. or 1., got: {ver}"
		);
		aphrodite_hermes_free_string(json_ptr);
	}

	#[test]
	fn test_list_tools_returns_array() {
		let json_ptr = aphrodite_hermes_list_tools();
		let json = unsafe { CStr::from_ptr(json_ptr) }.to_string_lossy().into_owned();
		let v: serde_json::Value = serde_json::from_str(&json).unwrap();
		assert!(v.is_array());
		assert!(v.as_array().unwrap().len() >= 10);
		aphrodite_hermes_free_string(json_ptr);
	}

	#[test]
	fn test_list_skills_returns_array() {
		let json_ptr = aphrodite_hermes_list_skills();
		let json = unsafe { CStr::from_ptr(json_ptr) }.to_string_lossy().into_owned();
		let v: serde_json::Value = serde_json::from_str(&json).unwrap();
		assert!(v.is_array());
		aphrodite_hermes_free_string(json_ptr);
	}

	#[test]
	fn test_dispatch_unknown_tool() {
		let name = CString::new("nonexistent").unwrap();
		let args = CString::new("{}").unwrap();
		let result_ptr = aphrodite_hermes_dispatch_tool(name.as_ptr(), args.as_ptr());
		let result = unsafe { CStr::from_ptr(result_ptr) }.to_string_lossy().into_owned();
		assert!(result.contains("error"));
		aphrodite_hermes_free_string(result_ptr);
	}

	#[test]
	fn test_call_hook_session_start() {
		let _g = crate::test_guard();
		let hook = CString::new("session_start").unwrap();
		let args = CString::new("{}").unwrap();
		let result_ptr = aphrodite_hermes_call_hook(hook.as_ptr(), args.as_ptr());
		let result = unsafe { CStr::from_ptr(result_ptr) }.to_string_lossy().into_owned();
		let v: serde_json::Value = serde_json::from_str(&result).unwrap();
		assert_eq!(v["status"], "ok");
		aphrodite_hermes_free_string(result_ptr);
	}

	// ── 01-F3: the bridge's `pre_llm_call` arm - the only one Hermes calls
	// in production - must actually inject active directive text, not just
	// the catalog summary. Previously it called `session::catalog_summary`
	// directly and never touched `directives::build_directive_context` at
	// all, so the "wire directives into pre_llm_call" feature was dead
	// end-to-end despite `hooks::pre_llm_call` (core, unreachable from this
	// path) already building that context correctly. ──
	#[test]
	fn test_call_hook_pre_llm_call_injects_active_directive_context() {
		let _g = crate::test_guard();
		aphrodite_hermes_call_hook(
			CString::new("session_start").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		with_shared(|state| {
			state.directives.insert(
				"focus".into(),
				aphrodite::directives::Directive {
					name: "focus".into(),
					content: "stay concise, 1-2 tools/turn".into(),
				},
			);
			state.active_directives = vec!["focus".into()];
		});

		let hook_ptr = aphrodite_hermes_call_hook(
			CString::new("pre_llm_call").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		let result = unsafe { CStr::from_ptr(hook_ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(hook_ptr);

		let v: serde_json::Value = serde_json::from_str(&result).unwrap();
		let context = v["context"].as_str().unwrap_or_default();
		assert!(
			context.contains("[directives: focus]"),
			"context missing directive marker: {context}"
		);
		assert!(context.contains("stay concise"), "context missing directive body: {context}");

		// `active_directives` is process-global and not reset by
		// session_start (deliberately) - clean up so this test doesn't leak
		// an active directive into whichever test runs next.
		with_shared(|state| state.active_directives.clear());
	}

	// ── T13 (F11): the production hook-dispatch path Python actually calls
	// (`aphrodite_hermes_call_hook("post_llm_call", ...)`) must archive the
	// turn's marker, not just advance the counter - a prior fix routed
	// `hooks::post_llm_call` through `crates/aphrodite/src/lib.rs`'s
	// separate FFI dispatch, but this crate's own `call_hook` bypassed it
	// entirely by calling `next_turn` directly, so `aphrodite_diff` still
	// returned zero turns in real Hermes usage even after that fix. ──
	#[test]
	fn test_call_hook_post_llm_call_archives_turn_for_aphrodite_diff() {
		let _g = crate::test_guard();
		aphrodite_hermes_call_hook(
			CString::new("session_start").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);

		let compress_args = CString::new(serde_json::json!({"content": "x".repeat(5000)}).to_string()).unwrap();
		let compress_ptr = aphrodite_hermes_dispatch_tool(
			CString::new("aphrodite_compress").unwrap().as_ptr(),
			compress_args.as_ptr(),
		);
		unsafe { CStr::from_ptr(compress_ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(compress_ptr);

		let hook_ptr = aphrodite_hermes_call_hook(
			CString::new("post_llm_call").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		aphrodite_hermes_free_string(hook_ptr);

		let diff_ptr = aphrodite_hermes_dispatch_tool(
			CString::new("aphrodite_diff").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		let diff_result = unsafe { CStr::from_ptr(diff_ptr) }.to_string_lossy().into_owned();
		let v: serde_json::Value = serde_json::from_str(&diff_result).unwrap();
		aphrodite_hermes_free_string(diff_ptr);

		assert_eq!(v["total"], 1, "aphrodite_diff must report the archived turn: {v:?}");
	}

	// ── 01-F1: `transform_tool_result` is the hook Hermes fires on EVERY
	// tool result, automatically - not the rarely-called `aphrodite_compress`
	// tool. It must unwrap Hermes JSON wrappers just like the tool path does,
	// or every automatic compression of a wrapped terminal/search/patch
	// result regresses to a useless "[json:1items 1L]" preview.
	#[test]
	fn test_call_hook_transform_tool_result_unwraps_hermes_wrapper() {
		let _g = crate::test_guard();
		aphrodite_hermes_call_hook(
			CString::new("session_start").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);

		let wrapped = serde_json::json!({"output": "x".repeat(5000), "exit_code": 1}).to_string();
		let args = serde_json::json!({"tool_name": "terminal", "result": wrapped}).to_string();
		let hook_ptr = aphrodite_hermes_call_hook(
			CString::new("transform_tool_result").unwrap().as_ptr(),
			CString::new(args).unwrap().as_ptr(),
		);
		let hook_result = unsafe { CStr::from_ptr(hook_ptr) }.to_string_lossy().into_owned();
		let marker_str: String = serde_json::from_str(&hook_result).expect("a marker string, not null");
		aphrodite_hermes_free_string(hook_ptr);

		assert!(
			!marker_str.contains("[json:"),
			"hook path must unwrap the Hermes wrapper for preview, got marker: {marker_str}"
		);

		let (hash, preview) = with_shared(|state| {
			let last = state.recent_markers.last().expect("hook must record a marker");
			(last.hash.clone(), last.preview.clone())
		});
		assert!(
			!preview.starts_with("[json:"),
			"recorded preview must reflect the unwrapped payload: {preview}"
		);

		let retrieved = crate::tools::dispatch("aphrodite_retrieve", &serde_json::json!({"hash": hash}).to_string());
		assert_eq!(retrieved["found"], true);
		assert_eq!(
			retrieved["content"].as_str().unwrap(),
			wrapped,
			"retrieve must return the original wrapper losslessly, not just the extracted output"
		);
	}

	#[test]
	fn test_get_schema_known_tool() {
		let name = CString::new("aphrodite_compress").unwrap();
		let result_ptr = aphrodite_hermes_get_schema(name.as_ptr());
		let result = unsafe { CStr::from_ptr(result_ptr) }.to_string_lossy().into_owned();
		let v: serde_json::Value = serde_json::from_str(&result).unwrap();
		assert_eq!(v["name"], "aphrodite_compress");
		aphrodite_hermes_free_string(result_ptr);
	}

	// ── T2 (F1): a panic anywhere inside a `guarded()` body must surface as
	// an error JSON, not unwind across the extern "C" boundary. ──
	#[test]
	fn test_guarded_converts_panic_to_error_json() {
		let ptr = guarded(|| panic!("deliberate test panic"));
		let json = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		let v: serde_json::Value = serde_json::from_str(&json).unwrap();
		assert!(v["error"].as_str().unwrap().contains("panicked"));
		aphrodite_hermes_free_string(ptr);
	}

	#[test]
	fn test_call_hook_panic_path_returns_error_not_abort() {
		// dispatch_tool with a tool name that panics in the registry lookup
		// path would abort pre-T2; now it must come back as error JSON.
		let name = CString::new("aphrodite_compress").unwrap();
		// Malformed args (missing required fields) exercise the same guarded
		// path without relying on a specific internal panic site.
		let args = CString::new("not json").unwrap();
		let ptr = aphrodite_hermes_dispatch_tool(name.as_ptr(), args.as_ptr());
		assert!(!ptr.is_null());
		aphrodite_hermes_free_string(ptr);
	}
}
