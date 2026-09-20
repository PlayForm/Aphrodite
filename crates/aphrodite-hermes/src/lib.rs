//! aphrodite-hermes: Hermes Agent-specific integration crate.
//!
//! This crate handles all Hermes-specific concerns - tool schemas,
//! hook dispatch, directive provisioning - leaving the core `aphrodite`
//! crate as a pure, agent-agnostic compression engine. (Bundled skill
//! registration is the Python plugin's job, not this crate's.)
//!
//! Architecture:
//!   Python plugin (thin loader) → ctypes → libaphrodite_hermes.dylib
//!                                           ├─ Tool dispatch (compress, retrieve, stats, etc.)
//!                                           ├─ Hook dispatch (on_session_start, transform, terminal, pre/post LLM)
//!                                           ├─ Directive provisioning (materialize the
//!                                           │  embedded builtin set into the runtime home)
//!                                           ↓ (depends on)
//!                                    aphrodite crate (rlib)
//!                                           ├─ Core compression (hooks, state, marker)
//!                                           ├─ Resolution (resolve, stage2, struct)
//!                                           └─ Catalog, session, prefetch, config, chain_split

// See crates/aphrodite/src/lib.rs's matching comment: fixing this properly
// (marking every `pub extern "C" fn` `unsafe`) is report 03's job, not a
// side effect of wiring up a CI clippy gate.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

mod debug;
mod directives;
mod schemas;
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
	static STATE:OnceLock<Mutex<AphroditeState>> = OnceLock::new();
	STATE.get_or_init(|| {
		#[cfg(not(test))]
		let state = {
			let mut s = AphroditeState::default();
			let cfg = aphrodite::config_loader::Config::load();
			cfg.apply_compression(&mut s);
			// Issue #11 WS4: also push `[previews] preview_max_chars` into
			// the process-global preview builder so the dylib path caps
			// previews exactly like the proxy path (the key was
			// declared-but-unread dead config). Precedence is
			// $APHRODITE_PREVIEW_MAX_CHARS > TOML > default 120; an absent
			// key means unlimited.
			cfg.apply_previews();
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
pub(crate) fn with_shared<T>(f:impl FnOnce(&mut AphroditeState) -> T) -> T {
	let mut guard = shared().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	f(&mut guard)
}

/// Serializes tests that assert across multiple state-mutating calls, since
/// the shared session state is process-global and `on_session_start` resets it.
#[cfg(test)]
pub(crate) fn test_guard() -> std::sync::MutexGuard<'static, ()> {
	static G:OnceLock<Mutex<()>> = OnceLock::new();
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
pub(crate) fn replacement_from(r:&serde_json::Value, session:&str) -> serde_json::Value {
	if r.get("chain_split").and_then(|v| v.as_bool()).unwrap_or(false) {
		// Fine-grained chain split, invisibility contract: the LLM must see
		// exactly what it would for any compressed output - the NATURAL
		// per-segment CCR markers (each segment's own `<<<CCR:hash|type|size>>>`
		// marker, retrievable by its own hash), one per line. Never the
		// `summary` string: it announces the mechanism (`[chain:N segs | ...]`)
		// and stays in the JSON payload for telemetry/metrics only.
		if let Some(markers) = r.get("markers").and_then(|v| v.as_array()) {
			let joined:Vec<String> = markers
				.iter()
				.filter_map(|m| m.get("marker").and_then(|v| v.as_str()).map(str::to_string))
				.collect();
			if !joined.is_empty() {
				return serde_json::Value::String(joined.join("\n"));
			}
		}
	}
	if r.get("compressed").and_then(|v| v.as_bool()).unwrap_or(false)
		&& let Some(marker) = r.get("marker").and_then(|v| v.as_str())
	{
		// Per-session debug (flag file): prepend a `[aphrodite-debug ...]`
		// line before the CCR marker when the toggle is on for this session,
		// so the session sees what the engine did without retrieving. Quiet
		// path (flag off) costs one stat() and returns the marker unchanged.
		let mut out = String::with_capacity(marker.len() + 128);
		if let Some(line) = debug::debug_line(r, session) {
			out.push_str(&line);
			out.push('\n');
		}
		out.push_str(marker);
		return serde_json::Value::String(out);
	}
	serde_json::Value::Null
}

/// Default cache proxy port, used when `APHRODITE_CACHE_PORT` is unset.
const DEFAULT_CACHE_PORT:u16 = 9797;
/// Default token proxy port, used when `APHRODITE_TOKEN_PORT` is unset.
const DEFAULT_TOKEN_PORT:u16 = 9798;

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
	let port_from_env = |var:&str, default:u16| {
		match std::env::var(var) {
			Ok(v) => {
				match v.parse::<u16>() {
					Ok(port) => port,
					Err(_) => {
						eprintln!(
							"aphrodite-hermes: {}={:?} is not a valid port (1-65535); using default {}",
							var, v, default
						);
						default
					},
				}
			},
			Err(_) => default,
		}
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
	let alive = |addr:String| {
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
unsafe fn cstr_to_string(ptr:*const c_char) -> String {
	if ptr.is_null() {
		String::new()
	} else {
		// SAFETY: `ptr` is non-null here, and per the safety contract of this
		// function it points to a valid, NUL-terminated C string that stays
		// valid for the duration of this call.
		unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
	}
}

fn to_c_string(s:&str) -> *mut c_char { CString::new(s).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut()) }

fn to_json_error(msg:&str) -> *mut c_char { to_c_string(&serde_json::json!({"error": msg}).to_string()) }

/// Run `f` under `catch_unwind`, converting a panic into an error-JSON string
/// instead of letting it unwind across the `extern "C"` boundary (which would
/// otherwise trigger the Rust runtime's forced process abort - this crate had
/// zero panic guards until this was added). Every exported fn except
/// `aphrodite_hermes_free_string` (must stay minimal/infallible) routes
/// through this.
fn guarded(f:impl FnOnce() -> *mut c_char + std::panic::UnwindSafe) -> *mut c_char {
	std::panic::catch_unwind(f).unwrap_or_else(|_| to_json_error("internal error: panicked in aphrodite-hermes"))
}

// ── Tool dispatch C ABI ────────────────────────────────────

/// Dispatch an aphrodite tool call by name.
/// Returns JSON result string. Caller must free with
/// aphrodite_hermes_free_string.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_dispatch_tool(tool_name:*const c_char, args_json:*const c_char) -> *mut c_char {
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
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_list_tools() -> *mut c_char {
	guarded(|| {
		let schemas = schemas::all_schemas();
		to_c_string(&serde_json::to_string(&schemas).unwrap_or_default())
	})
}

/// Get a single tool schema by name.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_get_schema(tool_name:*const c_char) -> *mut c_char {
	let name = unsafe { cstr_to_string(tool_name) };
	guarded(std::panic::AssertUnwindSafe(move || {
		match schemas::get_schema(&name) {
			Some(s) => to_c_string(&serde_json::to_string(&s).unwrap_or_default()),
			None => to_json_error(&format!("unknown tool: {}", name)),
		}
	}))
}

/// Free a string returned by any aphrodite_hermes_* function.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_free_string(s:*mut c_char) {
	if !s.is_null() {
		unsafe {
			let _ = CString::from_raw(s);
		}
	}
}

/// Version of this crate.
#[unsafe(no_mangle)]
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
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_call_hook(hook_name:*const c_char, args_json:*const c_char) -> *mut c_char {
	let name = unsafe { cstr_to_string(hook_name) };
	let args = unsafe { cstr_to_string(args_json) };

	guarded(std::panic::AssertUnwindSafe(move || {
		// Parse args as JSON object
		let parsed:serde_json::Value = match serde_json::from_str(&args) {
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

		let result:serde_json::Value = with_shared(|state| {
			match name.as_str() {
				// Accept both the canonical Hermes name and the legacy alias.
				"on_session_start" | "session_start" => aphrodite::session::on_session_start(state),
				"pre_tool_call" => {
					// ── Auto-backgrounding: intercept BEFORE execution ──
					// Returns {"action":"modify","args":{"background":true,"notify_on_complete":true}}
					// so Hermes runs the tool in background instead of blocking.
					if state.poll_worker_enabled {
						let call_tool = parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
						if call_tool == "terminal" || call_tool == "process" {
							let command = parsed.get("args").and_then(|a| a.get("command")).and_then(|v| v.as_str());
							// For `process(action='poll')`, don't background - it's a check call.
							let is_poll = call_tool == "process"
								&& parsed
									.get("args")
									.and_then(|a| a.get("action"))
									.and_then(|v| v.as_str())
									.map(|a| a == "poll")
									.unwrap_or(false);
							if !is_poll
								&& let Some((_task_id, cmd_summary)) =
									aphrodite::poll_worker::should_background_pre(command)
							{
								// We don't create a BgTask here - Hermes handles the
								// process lifecycle. We'll track completion via
								// transform_tool_result when the agent polls.
								return serde_json::json!({
									"action": "modify",
									"args": {
										"background": true,
										"notify_on_complete": true,
									},
									"message": format!(
										"aphrodite: auto-backgrounding `{}`", cmd_summary
									),
								});
							}
						}
					}
					// ── Fine-grained chain splitting: rewrite chained commands ──
					// LLMs chain (`cd x && cargo build && cargo test`) into one call;
					// rewriting with segment markers lets transform_tool_result split
					// the output into per-segment CCR entries (N compact previews).
					// Tier 1 teaching loop: only chains with at least
					// `chain_split_min_segments` segments are rewritten - the
					// threshold adapts from retrieval consequences (invisible).
					if state.chain_split_enabled {
						let call_tool = parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
						if call_tool == "terminal"
							&& let Some(command) =
								parsed.get("args").and_then(|a| a.get("command")).and_then(|v| v.as_str())
							&& let Some(segments) = aphrodite::chain_split::split_chain(command)
							&& segments.len() >= state.chain_split_min_segments
						{
							let rewritten = aphrodite::chain_split::build_marked_command(&segments);
							if rewritten != command {
								let mut args = parsed.get("args").cloned().unwrap_or_else(|| serde_json::json!({}));
								args["command"] = serde_json::Value::String(rewritten.clone());
								return serde_json::json!({
									"action": "modify",
									"args": args,
									"message": format!(
										"aphrodite: split chained command into {} segments (fine-grained CCR)",
										segments.len()
									),
								});
							}
						}
					}
					serde_json::Value::Null // pass through
				},
				"transform_tool_result" => {
					// Poll-result tracking: update running BgTasks from
					// process(action='poll') results. Only active when
					// poll_worker is enabled.
					if state.poll_worker_enabled && tool == "process" {
						aphrodite::poll_worker::update_from_poll(state, tool, tool_content);
					}

					// Hermes wraps every tool result in JSON (`{"output":...,
					// "exit_code":...}`, `{"total_count":...,"matches":[...]}`,
					// etc.) - unwrap it to classify/preview the real payload,
					// but hand core the ORIGINAL content so it's what gets
					// hashed and stored (retrieval must stay lossless).
					let classify = crate::tools::unwrap_hermes_result(tool_content);
					// 05-P2/T8: parse the error/args/timing signals Hermes ships
					// on every call (previously dropped on the floor) and route
					// them into the telemetry ring via `_with_meta`.
					let meta = aphrodite::hooks::ToolCallMeta {
						args_json:parsed.get("args"),
						status:parsed.get("status").and_then(|v| v.as_str()),
						error_type:parsed.get("error_type").and_then(|v| v.as_str()),
						error_message:parsed.get("error_message").and_then(|v| v.as_str()),
						duration_ms:parsed.get("duration_ms").and_then(|v| v.as_u64()),
					};
					let r = aphrodite::hooks::transform_tool_result_with_meta(
						state,
						tool_content,
						tool,
						classify.as_ref().map(|(c, t)| (c.as_str(), t.as_str())),
						&meta,
					);
					// Per-session debug: the caller's session_id rides in the
					// hook kwargs - thread it so the flag resolves against the
					// session tree (root scoped flag wins, global fallback).
					let sid = parsed.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
					replacement_from(&r, sid)
				},
				"transform_terminal_output" => {
					let classify = crate::tools::unwrap_hermes_result(term_content);
					// 05-P2/T8: thread the terminal `command`/`returncode` into
					// the telemetry ring so terminal failures feed error-loop
					// detection.
					let command = parsed.get("command").and_then(|v| v.as_str());
					let returncode = parsed.get("returncode").and_then(|v| v.as_i64());
					let r = aphrodite::hooks::transform_terminal_output_with_meta(
						state,
						term_content,
						classify.as_ref().map(|(c, t)| (c.as_str(), t.as_str())),
						command,
						returncode,
					);
					// Hermes does NOT thread session_id through
					// transform_terminal_output (terminal_tool_result.py:144
					// passes only command/output/returncode/task_id/env_type) -
					// fall back to the current turn's session so terminal
					// output keeps the same session scope as tool results.
					let sid = parsed.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
					let sid = if sid.is_empty() { crate::debug::last_session() } else { sid.to_string() };
					replacement_from(&r, &sid)
				},
				"pre_llm_call" => {
					// 05-P1/T1: route this - the ONLY pre_llm_call arm Hermes
					// actually calls in production - through the single shared
					// `flow::build_turn_context` assembler, the same one core's
					// `hooks::pre_llm_call` and `context_engine_pre_llm` use.
					// This makes the G1 bug class (bridge reimplements the hook
					// body and forks on every core improvement - directives were
					// dead here for two releases, 01-F3) unrepresentable: there
					// is exactly one place that composes model-visible context.
					// `args.len()` is the request-size proxy for the P9 telemetry
					// line (Hermes serializes conversation_history into the
					// kwargs JSON).
					// Per-session debug: pre_llm_call is the ONE hook Hermes
					// threads `parent_session_id` through (turn_context.py:
					// 698-706), so record session -> parent here to build the
					// tree the debug flag resolves against.
					if let (Some(sid), Some(pid)) = (
						parsed.get("session_id").and_then(|v| v.as_str()),
						parsed.get("parent_session_id").and_then(|v| v.as_str()),
					) {
						debug::record_session(sid, pid);
					}
					let context = aphrodite::flow::build_turn_context(state, Some(args.len()));
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
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_get_schemas() -> *mut c_char {
	guarded(|| {
		let schemas = schemas::all_schemas();
		to_c_string(&serde_json::json!(schemas).to_string())
	})
}

/// Return hook names as a JSON array.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_get_hooks() -> *mut c_char {
	// Hermes invokes the session hook as `on_session_start` (the `on_` prefix is
	// required by its VALID_HOOKS table); registering `session_start` silently
	// no-ops. The other four names match Hermes verbatim.
	guarded(|| {
		to_c_string(
			&serde_json::json!([
				"on_session_start",
				"pre_tool_call",
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
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_proxy_health() -> *mut c_char {
	guarded(|| to_c_string(&proxy_health().to_string()))
}

// ── Directives provisioning C ABI ──────────────────────────

/// Materialize the embedded behavioral directives into
/// `<runtime-home>/directives/` (idempotent; never overwrites user data).
///
/// The shipped directive set is embedded in the core `aphrodite` crate
/// (`builtin_directives/*.md` via `include_str!`, exposed through
/// `aphrodite::directives::loaded_builtins()`); this fn provisions it into
/// the user-data home so the plugin directory never holds runtime state.
///
/// `home_dir` is the runtime home (the user-data folder, e.g.
/// `~/.hermes/aphrodite`). If null/empty it is resolved from
/// `$APHRODITE_DIRECTIVES_DIR` (exact dir), `$APHRODITE_HOME` (home), then
/// `$HOME/.hermes/aphrodite`. Returns JSON
/// `{"status":"ok","dir":...,"written":[...],"skipped":[...],"warnings":[...]}`
/// - always `status:"ok"` (failures degrade to warnings). Caller must free
///   with `aphrodite_hermes_free_string`.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_materialize_directives(home_dir:*const c_char) -> *mut c_char {
	let home = unsafe { cstr_to_string(home_dir) };
	guarded(std::panic::AssertUnwindSafe(move || {
		let result = directives::materialize_into(&home);
		to_c_string(&serde_json::to_string(&result).unwrap_or_default())
	}))
}

#[cfg(test)]
mod tests {
	use std::ffi::CString;

	use super::*;

	/// Serializes tests that touch process-global env vars
	/// (`APHRODITE_CACHE_PORT`/`APHRODITE_TOKEN_PORT`).
	fn env_guard() -> std::sync::MutexGuard<'static, ()> {
		static G:std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
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
		unsafe { std::env::set_var("APHRODITE_CACHE_PORT", "not-a-port") };
		unsafe { std::env::remove_var("APHRODITE_TOKEN_PORT") };
		let (cache, token) = configured_ports();
		unsafe { std::env::remove_var("APHRODITE_CACHE_PORT") };
		assert_eq!(cache, DEFAULT_CACHE_PORT);
		assert_eq!(token, DEFAULT_TOKEN_PORT);
	}

	#[test]
	fn test_configured_ports_honors_valid_override() {
		let _g = env_guard();
		unsafe { std::env::set_var("APHRODITE_CACHE_PORT", "19797") };
		let (cache, _token) = configured_ports();
		unsafe { std::env::remove_var("APHRODITE_CACHE_PORT") };
		assert_eq!(cache, 19797);
	}

	// ── Chain-split invisibility contract: the LLM sees the natural
	// per-segment CCR markers (joined, one per line), never the `summary`
	// string that announces the mechanism. ──
	#[test]
	fn test_replacement_from_chain_split_joins_natural_markers() {
		let r = serde_json::json!({
			"status": "ok",
			"compressed": true,
			"chain_split": true,
			"segments": 2,
			"summary": "[chain:2 segs | 200 orig → 90 markers]",
			"markers": [
				{
					"index": 0,
					"type": "terminal",
					"size": 120,
					"hash": "aaaa",
					"preview": "[terminal:120B] seg one",
					"marker": "<<<CCR:aaaa|terminal|120>>>\n[terminal:120B] seg one"
				},
				{
					"index": 1,
					"type": "build",
					"size": 80,
					"hash": "bbbb",
					"preview": "[build:80B] seg two",
					"marker": "<<<CCR:bbbb|build|80>>>\n[build:80B] seg two"
				}
			]
		});
		let out = replacement_from(&r, "test-session");
		let s = out.as_str().expect("chain_split must yield a string");
		// Natural markers, one per line.
		assert!(s.starts_with("<<<CCR:aaaa|terminal|120>>>"));
		assert!(s.contains("\n<<<CCR:bbbb|build|80>>>"));
		assert!(s.contains("[terminal:120B] seg one"));
		assert!(s.contains("[build:80B] seg two"));
		// The mechanism is invisible: no summary string, no 'chain' word.
		assert!(!s.contains("[chain:"));
		assert!(!s.contains("chain"));
		assert!(!s.contains("orig →"));
	}

	#[test]
	fn test_replacement_from_non_chain_uses_single_marker() {
		let r = serde_json::json!({
			"status": "ok",
			"compressed": true,
			"hash": "cccc",
			"type": "text",
			"size": 42,
			"preview": "[text:42B] hi",
			"marker": "<<<CCR:cccc|text|42>>>\n[text:42B] hi"
		});
		let s = replacement_from(&r, "test-session");
		assert_eq!(s.as_str().unwrap(), "<<<CCR:cccc|text|42>>>\n[text:42B] hi");
	}

	#[test]
	fn test_version_is_semver() {
		let json_ptr = aphrodite_hermes_version();
		let json = unsafe { CStr::from_ptr(json_ptr) }.to_string_lossy().into_owned();
		let v:serde_json::Value = serde_json::from_str(&json).unwrap();
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
		let v:serde_json::Value = serde_json::from_str(&json).unwrap();
		assert!(v.is_array());
		assert!(v.as_array().unwrap().len() >= 10);
		aphrodite_hermes_free_string(json_ptr);
	}

	// ── F4 hardening: the FFI allocate/free pairing must be symmetric and
	// allocator-safe. Every string returned across the C ABI is allocated
	// with `CString::new(...).into_raw()` (`to_c_string`) and reclaimed by
	// `CString::from_raw` inside `aphrodite_hermes_free_string` - never via a
	// manual `libc::free`/`free()` or an allocator-specific dealloc. Because
	// both halves route through the Rust global allocator (the crate installs
	// no `#[global_allocator]`, so that is the default system allocator shared
	// process-wide), the pairing holds regardless of which dylib image
	// allocated and which freed it, and is immune to hot-reload allocator
	// skew. This test round-trips fresh allocations (allocate → read back →
	// free) repeatedly; an asymmetric pairing (wrong deallocator, double
	// free, invalid free) would abort the harness, not silently pass. ──
	#[test]
	fn test_free_string_round_trip_symmetric() {
		// Varying lengths incl. empty string: allocate, read, free, repeat.
		for i in 0..256 {
			let payload = format!("round-trip-{i}-{}", "x".repeat(i % 64));
			let ptr = to_c_string(&payload);
			assert!(!ptr.is_null(), "to_c_string returned null for payload #{i}");
			let read = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
			assert_eq!(read, payload, "read-back mismatch for payload #{i}");
			aphrodite_hermes_free_string(ptr);
		}
		// Empty string must round-trip too (CString is NUL-terminated).
		let ptr = to_c_string("");
		assert!(!ptr.is_null());
		assert_eq!(unsafe { CStr::from_ptr(ptr) }.to_bytes(), b"");
		aphrodite_hermes_free_string(ptr);
		// Error-JSON path (`to_json_error` → `to_c_string`) pairs identically.
		for i in 0..64 {
			let ptr = to_json_error(&format!("boom-{i}"));
			assert!(!ptr.is_null());
			let read = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
			assert!(read.contains("boom-"), "error payload #{i} corrupted");
			aphrodite_hermes_free_string(ptr);
		}
		// Interior NUL: CString::new fails → null; free_string(null) is a no-op.
		let ptr = to_c_string("a\0b");
		assert!(ptr.is_null());
		aphrodite_hermes_free_string(ptr);
		aphrodite_hermes_free_string(std::ptr::null_mut());
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
		let v:serde_json::Value = serde_json::from_str(&result).unwrap();
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
				aphrodite::directives::Directive { name:"focus".into(), content:"stay concise, 1-2 tools/turn".into() },
			);
			state.active_directives = vec!["focus".into()];
		});

		let hook_ptr = aphrodite_hermes_call_hook(
			CString::new("pre_llm_call").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		let result = unsafe { CStr::from_ptr(hook_ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(hook_ptr);

		let v:serde_json::Value = serde_json::from_str(&result).unwrap();
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

	// ── 05-T1 (P1): the end-to-end proof that directives inject through the
	// single `flow::build_turn_context` assembler on the production Hermes
	// path. Activate a directive, call the `pre_llm_call` hook, and assert the
	// `[directives:` marker appears in the returned `context`. This is the
	// choke point that makes the G1 bug class (directives dead on the Hermes
	// path) unrepresentable. ──
	#[test]
	fn test_call_hook_pre_llm_includes_directives() {
		let _g = crate::test_guard();
		aphrodite_hermes_call_hook(
			CString::new("session_start").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		with_shared(|state| {
			state.directives.insert(
				"focus".into(),
				aphrodite::directives::Directive { name:"focus".into(), content:"stay targeted, 1-2 tools".into() },
			);
			state.active_directives = vec!["focus".into()];
		});

		let hook_ptr = aphrodite_hermes_call_hook(
			CString::new("pre_llm_call").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		let result = unsafe { CStr::from_ptr(hook_ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(hook_ptr);

		let v:serde_json::Value = serde_json::from_str(&result).unwrap();
		let context = v["context"].as_str().unwrap_or_default();
		assert!(
			context.contains("[directives: focus]"),
			"assembler must inject directives on the Hermes path: {context}"
		);
		assert!(context.contains("stay targeted"), "directive body missing: {context}");

		with_shared(|state| state.active_directives.clear());
	}

	// ── 05-T8 (P2): the bridge must parse the status/error kwargs Hermes ships
	// on a tool result and record a failing telemetry event. ──
	#[test]
	fn test_call_hook_tool_result_records_error_event() {
		let _g = crate::test_guard();
		aphrodite_hermes_call_hook(
			CString::new("session_start").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		let args = serde_json::json!({
			"tool_name": "terminal",
			"result": "error[E0382]: borrow of moved value\n".repeat(50),
			"status": "error",
			"error_type": "compile_error",
			"error_message": "E0382: borrow of moved value",
			"args": {"command": "cargo build"},
		})
		.to_string();
		let hook_ptr = aphrodite_hermes_call_hook(
			CString::new("transform_tool_result").unwrap().as_ptr(),
			CString::new(args).unwrap().as_ptr(),
		);
		aphrodite_hermes_free_string(hook_ptr);

		with_shared(|state| {
			let ev = state.tool_events.back().expect("bridge must record a tool event");
			assert!(!ev.ok, "status=error must record a failing event");
			assert!(ev.error_sig.is_some(), "a failing event must carry an error_sig");
		});
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
		let v:serde_json::Value = serde_json::from_str(&diff_result).unwrap();
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
		let marker_str:String = serde_json::from_str(&hook_result).expect("a marker string, not null");
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
		let v:serde_json::Value = serde_json::from_str(&result).unwrap();
		assert_eq!(v["name"], "aphrodite_compress");
		aphrodite_hermes_free_string(result_ptr);
	}

	// ── T2 (F1): a panic anywhere inside a `guarded()` body must surface as
	// an error JSON, not unwind across the extern "C" boundary. ──
	#[test]
	fn test_guarded_converts_panic_to_error_json() {
		let ptr = guarded(|| panic!("deliberate test panic"));
		let json = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		let v:serde_json::Value = serde_json::from_str(&json).unwrap();
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
	// ── Poll-worker: pre_tool_call auto-backgrounding tests ──

	#[test]
	fn test_pre_tool_call_auto_backgrounds_terminal() {
		let _g = crate::test_guard();
		aphrodite_hermes_call_hook(
			CString::new("session_start").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		with_shared(|state| state.poll_worker_enabled = true);

		let args = serde_json::json!({
			"tool_name": "terminal",
			"args": {"command": "cargo build --release"},
		})
		.to_string();
		let ptr = aphrodite_hermes_call_hook(
			CString::new("pre_tool_call").unwrap().as_ptr(),
			CString::new(args).unwrap().as_ptr(),
		);
		let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(ptr);

		let v:serde_json::Value = serde_json::from_str(&result).unwrap();
		assert_eq!(v["action"], "modify", "pre_tool_call must return modify action: {result}");
		assert_eq!(v["args"]["background"], true);
		assert_eq!(v["args"]["notify_on_complete"], true);
	}

	#[test]
	fn test_pre_tool_call_does_not_background_poll_action() {
		let _g = crate::test_guard();
		aphrodite_hermes_call_hook(
			CString::new("session_start").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		with_shared(|state| state.poll_worker_enabled = true);

		let args = serde_json::json!({
			"tool_name": "process",
			"args": {"action": "poll"},
		})
		.to_string();
		let ptr = aphrodite_hermes_call_hook(
			CString::new("pre_tool_call").unwrap().as_ptr(),
			CString::new(args).unwrap().as_ptr(),
		);
		let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(ptr);

		assert_eq!(result, "null", "process poll must pass through unchanged");
	}

	#[test]
	fn test_pre_tool_call_disabled_passes_through() {
		let _g = crate::test_guard();
		aphrodite_hermes_call_hook(
			CString::new("session_start").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		with_shared(|state| state.poll_worker_enabled = false);

		let args = serde_json::json!({
			"tool_name": "terminal",
			"args": {"command": "cargo build --release"},
		})
		.to_string();
		let ptr = aphrodite_hermes_call_hook(
			CString::new("pre_tool_call").unwrap().as_ptr(),
			CString::new(args).unwrap().as_ptr(),
		);
		let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(ptr);

		assert_eq!(result, "null", "disabled flag must pass through: {result}");
	}

	// ── Tier 1 teaching loop: the adaptive split threshold gates the
	// rewrite - chains below `chain_split_min_segments` pass through
	// untouched (no markers), chains at/above it get rewritten. ──
	#[test]
	fn test_pre_tool_call_chain_split_respects_adaptive_threshold() {
		let _g = crate::test_guard();
		aphrodite_hermes_call_hook(
			CString::new("session_start").unwrap().as_ptr(),
			CString::new("{}").unwrap().as_ptr(),
		);
		with_shared(|state| {
			state.poll_worker_enabled = false;
			state.chain_split_enabled = true;
			// Threshold raised to 3: a 2-segment chain must NOT be split.
			state.chain_split_min_segments = 3;
		});

		// Below threshold: 2 segments → pass through (null).
		let args2 = serde_json::json!({
			"tool_name": "terminal",
			"args": {"command": "cd x && cargo build"},
		})
		.to_string();
		let ptr = aphrodite_hermes_call_hook(
			CString::new("pre_tool_call").unwrap().as_ptr(),
			CString::new(args2).unwrap().as_ptr(),
		);
		let result2 = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(ptr);
		assert_eq!(result2, "null", "2-segment chain below threshold must pass through: {result2}");

		// At/above threshold: 3 segments → rewritten with markers.
		let args3 = serde_json::json!({
			"tool_name": "terminal",
			"args": {"command": "cd x && cargo build && cargo test"},
		})
		.to_string();
		let ptr = aphrodite_hermes_call_hook(
			CString::new("pre_tool_call").unwrap().as_ptr(),
			CString::new(args3).unwrap().as_ptr(),
		);
		let result3 = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(ptr);
		let v3:serde_json::Value = serde_json::from_str(&result3).unwrap();
		assert_eq!(v3["action"], "modify", "3-segment chain must be rewritten: {result3}");
		assert!(
			v3["args"]["command"].as_str().unwrap().contains("__APHRODITE_SEG__"),
			"rewritten command must carry segment markers: {result3}"
		);
	}
}
