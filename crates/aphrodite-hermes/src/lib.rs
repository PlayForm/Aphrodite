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
	collections::HashMap,
	ffi::{CStr, CString},
	os::raw::c_char,
	sync::{
		Arc,
		Mutex,
		OnceLock,
		atomic::{AtomicBool, Ordering},
	},
};

// Re-export core aphrodite types for convenience
pub use aphrodite::state::AphroditeState;

// ── Process-global session-state registry ──────────────────
//
// Hermes runs a SHARED-GATEWAY plugin model: ONE dylib instance serves ALL
// sessions (parent + subagents) in the gateway process. A single
// process-global `AphroditeState` (the pre-fix model) meant a heavy hook in
// ONE session (subagent `pre_llm_call` -> `build_turn_context`) held the
// only lock; when Hermes' `plugins.hook_callback_timeout` abandoned that
// worker thread the mutex stayed locked forever, and every later hook in
// EVERY session blocked on it (gateway log: "3 abandoned workers", permanent
// skip until restart). State is therefore keyed by session_id: each session
// owns an `Arc<Mutex<AphroditeState>>` cloned from a config-seeded template,
// and hooks/tools lock ONLY their own session's mutex with a BOUNDED
// try_lock (a wedged session degrades fail-open, never blocks the gateway).
//
// The registry mutex itself is held only for the HashMap get-or-create -
// never while running hook work - so it cannot be wedged by an abandoned
// worker; it blocks with poison recovery like any plain static mutex.

/// Key for calls with no session context (tools, tests, empty session_id).
const DEFAULT_SESSION:&str = "default";

/// Config-seeded template state, built once per process.
///
/// Initializes from `aphrodite.toml` (report 07 F3/T16) via
/// `config_loader::Config` - previously this crate always ran on hardcoded
/// `AphroditeState::default()` values; `Config::load()`/`apply_compression`
/// were the only implementation of the advertised env>TOML>default
/// resolution for this path but had zero call sites anywhere in the repo.
/// Every new session state is a clone of this template, so a subagent spawn
/// inherits the same thresholds/directives/templates as the parent (the
/// single-session behavior is unchanged: the first session IS a clone of
/// the template, so one session behaves exactly as before).
///
/// Test builds skip the TOML load and use plain defaults: `Config::load()`
/// searches `./aphrodite.toml` first, and this crate's own tests run with
/// the workspace root (which HAS a real `aphrodite.toml`) as their working
/// directory - without this split, unit tests would non-hermetically pick
/// up whatever the repo's live config happens to contain instead of the
/// documented defaults they assert against.
fn config_template() -> &'static AphroditeState {
	static TEMPLATE:OnceLock<AphroditeState> = OnceLock::new();
	TEMPLATE.get_or_init(|| {
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
			// Pre-Rust port: register `[templates]` + `[previews]
			// model_family` so the dylib path renders TOML preview
			// templates exactly like the engine/proxy paths.
			cfg.apply_templates(&s);
			// Opt-in config auto-reload (`[compression] auto_reload = true`,
			// default off): watch aphrodite.toml and re-apply config fields
			// on change. Only config fields are mutated - session CCR state,
			// directive selection, and telemetry survive a reload (the
			// dropped dylib-binary hot-reload wiped them; fa85dfa).
			if s.auto_reload {
				spawn_config_watcher();
			}
			s
		};
		#[cfg(test)]
		let state = AphroditeState::default();
		state
	})
}

/// Per-session state registry: session_id -> Arc<Mutex<AphroditeState>>.
///
/// Gets-or-creates: a new session's state is a clone of [`config_template`]
/// (config-seeded in production, plain defaults in test builds), so every
/// session starts from the same config without re-reading aphrodite.toml.
/// Returns an owned `Arc` clone so the registry lock is dropped before the
/// caller contends on the session's state mutex.
fn session_state(sid:&str) -> Arc<Mutex<AphroditeState>> {
	let key = if sid.is_empty() { DEFAULT_SESSION } else { sid };
	let mut map = SESSIONS
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	map.entry(key.to_string())
		.or_insert_with(|| Arc::new(Mutex::new(config_template().clone())))
		.clone()
}

static SESSIONS:OnceLock<Mutex<HashMap<String, Arc<Mutex<AphroditeState>>>>> = OnceLock::new();

/// Bounded lock acquisition: try_lock in a loop bounded by a ~100ms
/// wall-clock deadline (the spec's "about 50 x 2ms" budget; a wall-clock
/// deadline keeps the bound precise despite platform sleep-granularity
/// overshoot - macOS sleeps ~6ms for a 2ms request).
///
/// Returns `None` when the mutex is wedged by an abandoned worker thread
/// (the gateway-hang root cause: Hermes' `plugins.hook_callback_timeout`
/// abandoned a worker that still held the old single global lock), so the
/// caller degrades fail-open instead of blocking past the hook timeout.
/// Poison is recovered like the old `.lock()` path: a panic in one call
/// must not wedge every later call.
fn lock_bounded(state:&Arc<Mutex<AphroditeState>>) -> Option<std::sync::MutexGuard<'_, AphroditeState>> {
	let deadline = std::time::Instant::now() + std::time::Duration::from_millis(100);
	loop {
		match state.try_lock() {
			Ok(guard) => return Some(guard),
			// Poisoned: recover the guard (same contract as the pre-fix
			// `.unwrap_or_else(PoisonError::into_inner)` path).
			Err(std::sync::TryLockError::Poisoned(p)) => return Some(p.into_inner()),
			Err(std::sync::TryLockError::WouldBlock) => {
				if std::time::Instant::now() >= deadline {
					return None;
				}
				std::thread::sleep(std::time::Duration::from_millis(2));
			},
		}
	}
}

/// Run `f` against the given session's state under a BOUNDED lock.
///
/// Resolves the per-session state for `sid` (gets-or-creates) and locks ONLY
/// that session's mutex - a heavy hook in one session can never block
/// another session's hooks. `None` means the session's lock is wedged by an
/// abandoned worker; the caller MUST degrade fail-open (never block).
pub(crate) fn with_session<T>(sid:&str, f:impl FnOnce(&mut AphroditeState) -> T) -> Option<T> {
	let state = session_state(sid);
	lock_bounded(&state).map(|mut guard| f(&mut guard))
}

/// Run `f` against the session state under a bounded lock.
///
/// Compatibility shim for tools and tests that lack a `session_id`: resolves
/// `crate::debug::last_session()` (fallback to the [`DEFAULT_SESSION`] key
/// when empty) and locks ONLY that session with the bounded lock. On `None`
/// (wedged lock) it runs `f` against a THROWAWAY clone of the config
/// template so the call still returns quickly - fail-open, never block; the
/// mutation is discarded (the template itself is never mutated, so future
/// session seeds stay pristine).
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
	let sid = crate::debug::last_session();
	let sid = if sid.is_empty() { DEFAULT_SESSION.to_string() } else { sid };
	let state = session_state(&sid);
	if let Some(mut guard) = lock_bounded(&state) {
		return f(&mut guard);
	}
	// Wedged session lock: degrade on a throwaway clone rather than block.
	let mut scratch = config_template().clone();
	f(&mut scratch)
}

/// Re-apply a freshly loaded config to the session state, preserving the
/// runtime-owned fields. `apply_compression` re-probes the directives
/// directory and reseeds `active_directives` - the live session owns those
/// (runtime swap/add/remove via `aphrodite_directive`), so they are
/// snapshot/restored around the re-apply. CCR stores, telemetry, and every
/// other runtime field are untouched by `apply_compression` itself.
fn apply_config_reload(cfg:&aphrodite::config_loader::Config, state:&mut AphroditeState) {
	let saved_directives = std::mem::take(&mut state.directives);
	let saved_active = std::mem::take(&mut state.active_directives);
	cfg.apply_compression(state);
	state.directives = saved_directives;
	state.active_directives = saved_active;
	cfg.apply_previews();
	cfg.apply_templates(state);
}

/// Apply `f` to EVERY registered session under a BOUNDED lock, skipping
/// wedged sessions (an abandoned worker holds their mutex - a config reload
/// must never block the watcher; the reload reaches them on a later pass).
///
/// Returns true when any session still has `auto_reload` on after the pass
/// (the watcher keeps running only while at least one session wants it).
fn for_each_session(f:impl Fn(&mut AphroditeState)) -> bool {
	let map = SESSIONS
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	// Clone the Arcs so the registry lock is released before contending on
	// any session state mutex.
	let sessions:Vec<Arc<Mutex<AphroditeState>>> = map.values().cloned().collect();
	drop(map);
	let mut any_reload = false;
	for state in sessions {
		if let Some(mut guard) = lock_bounded(&state) {
			f(&mut guard);
			if guard.auto_reload {
				any_reload = true;
			}
		}
		// Wedged session: skip (fail-open - it stays degraded, never blocks).
	}
	any_reload
}

/// Spawn the opt-in config auto-reload watcher. Mirrors the engine binary's
/// watcher (`main.rs`): watch the config search dirs (cwd, then the user
/// runtime home - the same precedence as `Config::load()`) non-recursively,
/// debounce 500ms on a modify of `aphrodite.toml`, then reload.
///
/// A reload reaches EVERY registered session (each under its own bounded
/// lock, wedged ones skipped), so a config change applies to the parent and
/// all subagent sessions alike.
///
/// Failure policy (fail-open, `aphrodite-boundaries`): a broken file keeps
/// the previous values (like the engine watcher) and records the failure on
/// `config_error` so `aphrodite_stats` shows "defaults in effect (parse
/// failed)"; the loader's visible warn (stderr fallback when no tracing
/// subscriber exists) fires on its own. Setting `auto_reload = false` in the
/// file stops the watcher on the next successful reload.
fn spawn_config_watcher() {
	use notify::Watcher;
	let _ = std::thread::Builder::new()
		.name("aphrodite-config-watcher".into())
		.spawn(move || {
			let mut watch_dirs = vec![std::path::PathBuf::from(".")];
			// The runtime home is one shared decision (home::runtime_home:
			// $APHRODITE_HOME -> $HERMES_HOME -> $HOME -> platform default), so
			// the watcher follows the same home the plugin shim resolves - under
			// a non-default Hermes home it must watch <hermes-home>/aphrodite,
			// never $HOME/.hermes/aphrodite (issue 40).
			watch_dirs.push(aphrodite::home::runtime_home());
			let (tx, rx) = std::sync::mpsc::channel::<()>();
			let mut watcher = match notify::recommended_watcher(move |res:Result<notify::Event, notify::Error>| {
				if let Ok(ev) = res
					&& matches!(ev.kind, notify::EventKind::Modify(_))
					&& ev.paths.iter().any(|p| p.to_string_lossy().contains("aphrodite.toml"))
				{
					let _ = tx.send(());
				}
			}) {
				Ok(w) => w,
				Err(e) => {
					eprintln!("[aphrodite] config auto-reload watcher failed to start: {e}");
					return;
				},
			};
			for dir in &watch_dirs {
				if let Err(e) = watcher.watch(dir, notify::RecursiveMode::NonRecursive) {
					eprintln!("[aphrodite] config auto-reload: cannot watch {}: {e}", dir.display());
				}
			}
			while rx.recv().is_ok() {
				// Debounce: let the editor's write settle (same as the engine
				// watcher's 500ms), and drain events accumulated meanwhile.
				std::thread::sleep(std::time::Duration::from_millis(500));
				while rx.try_recv().is_ok() {}
				let cfg = aphrodite::config_loader::Config::load();
				if cfg.parse_failure.is_some() {
					// Broken file: keep the previous values, surface the
					// failure on EVERY session (the loader's warn already
					// fired); wedged sessions are skipped.
					for_each_session(|s| s.config_error = cfg.parse_failure.clone());
					continue;
				}
				let keep_watching = for_each_session(|s| apply_config_reload(&cfg, s));
				if !keep_watching {
					// `auto_reload` turned off in the file - stop watching
					// (takes effect on the next successful reload).
					return;
				}
			}
		});
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
/// `null` otherwise to leave the original output untouched. When the hook
/// auto-expanded (`state.auto_expand` on, content within the limit), the
/// resolved `expanded` string is returned instead of the marker - the
/// marker itself stays in the payload and the store, still resolvable via
/// `aphrodite_retrieve`.
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
	// Auto-expand (dylib-side consumer): the hook stored the content and
	// resolved it inline, carrying the full text back in `expanded` - return
	// that so no `<<<CCR:...>>>` marker reaches the model. A payload that
	// flags `auto_expanded` without a usable `expanded` string falls through
	// to the marker path below (degrade, never null-replace the output).
	if r.get("auto_expanded").and_then(|v| v.as_bool()).unwrap_or(false)
		&& let Some(expanded) = r.get("expanded").and_then(|v| v.as_str())
	{
		return serde_json::Value::String(expanded.to_string());
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

// ── DEV-ONLY env-gated debug logger bridge (temporary instrumentation) ───
//
// All logging lives in `aphrodite::debug` (core); these helpers build the
// session-init line and the post-tool outcome line for the hook arms.

/// Emitted once per process: the session-init/config-summary line (first
/// `on_session_start` or `pre_llm_call` that has a session id wins).
static INIT_EMITTED:AtomicBool = AtomicBool::new(false);

/// DEV-ONLY: emit the session-init/config-summary line exactly once per
/// process. Includes dylib version, root session id, the config summary
/// (thresholds, auto-expand, live container, recursion, directives,
/// templates), the proxy-side code_multiplier env override, and proxy
/// health.
fn log_session_init(state:&AphroditeState, session:&str) {
	if session.is_empty() || INIT_EMITTED.swap(true, Ordering::Relaxed) {
		return;
	}
	let mut kv = vec![
		("version", env!("CARGO_PKG_VERSION").to_string()),
		("session", session.to_string()),
	];
	kv.extend(aphrodite::debug::config_summary(state));
	// code_multiplier is proxy-side config; surface the env override only
	// (the dylib has no proxy thresholds loaded).
	if let Ok(cm) = std::env::var("APHRODITE_CODE_MULTIPLIER") {
		kv.push(("code_multiplier", cm));
	}
	kv.push(("proxy", proxy_health().to_string()));
	crate::debug::log_bridge(state, session, "init", &kv);
}

/// DEV-ONLY: post-tool outcome line built from the transform payload
/// (marker emitted with hash+type+size, auto-expanded bytes, live container
/// chunks, chain-split segments, or a passthrough reason) + result size +
/// elapsed microseconds.
fn post_tool_log(state:&AphroditeState, session:&str, r:&serde_json::Value, result_bytes:usize, elapsed_us:u64) {
	let mut kv = vec![
		("result_bytes", result_bytes.to_string()),
		("elapsed_us", elapsed_us.to_string()),
		(
			"compressed",
			r.get("compressed").and_then(|v| v.as_bool()).unwrap_or(false).to_string(),
		),
	];
	if let Some(reason) = r.get("reason").and_then(|v| v.as_str()) {
		kv.push(("reason", reason.to_string()));
	}
	if let Some(h) = r.get("hash").and_then(|v| v.as_str()) {
		kv.push(("hash", h.to_string()));
	}
	if let Some(t) = r.get("type").and_then(|v| v.as_str()) {
		kv.push(("type", t.to_string()));
	}
	if let Some(sz) = r.get("size").and_then(|v| v.as_u64()) {
		kv.push(("size", sz.to_string()));
	}
	if let Some(ch) = r.get("chunks").and_then(|v| v.as_u64()) {
		kv.push(("chunks", ch.to_string()));
	}
	if let Some(n) = r.get("segments").and_then(|v| v.as_u64()) {
		kv.push(("segments", n.to_string()));
	}
	if r.get("auto_expanded").and_then(|v| v.as_bool()).unwrap_or(false) {
		let bytes = r
			.get("expanded")
			.and_then(|v| v.as_str())
			.map(|e| e.len().to_string())
			.unwrap_or_else(|| "true".to_string());
		kv.push(("auto_expanded_bytes", bytes));
	}
	crate::debug::log_bridge(state, session, "post_tool", &kv);
}

/// DEV-ONLY: predict the compression path for a tool call (best-effort
/// label for the pre_tool debug line; the transform arms log the actual
/// outcome).
fn expected_path(state:&AphroditeState, tool:&str, parsed:&serde_json::Value) -> String {
	// Mirror of `hooks::ESSENTIAL_TOOLS` (dev-only log; kept in sync
	// deliberately - drift here only mislabels a debug line).
	const ESSENTIAL:&[&str] = &[
		"skill_view",
		"skills_list",
		"skill_manage",
		"memory",
		"session_search",
		"read_file",
		"read_terminal",
	];
	if ESSENTIAL.contains(&tool) {
		return "passthrough_essential".into();
	}
	if tool.starts_with("aphrodite_") || tool.starts_with("headroom") {
		return "passthrough_self".into();
	}
	if state.auto_expand {
		return "auto_expand".into();
	}
	if tool == "terminal" && state.chain_split_enabled {
		let command = parsed.get("args").and_then(|a| a.get("command")).and_then(|v| v.as_str());
		if command.is_some_and(|c| c.contains("&&")) {
			return "chain_split".into();
		}
	}
	if state.live_container {
		return "live_container".into();
	}
	"compress_or_passthrough".into()
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

/// Fail-open degrade value for a hook whose session lock is wedged by an
/// abandoned worker (the bounded lock returned None). Never blocks and never
/// erases output: `pre_tool_call` -> null (no directive; the tool proceeds
/// normally), `transform_tool_result` / `transform_terminal_output` -> the
/// ORIGINAL content string (Hermes honors a string return as the
/// replacement; returning the identical content is a visible no-op),
/// `pre_llm_call` -> null (no context injection), lifecycle hooks -> null
/// (return ignored).
fn fail_open_hook(name:&str, tool_content:&str, term_content:&str) -> serde_json::Value {
	match name {
		"transform_tool_result" => serde_json::Value::String(tool_content.to_string()),
		"transform_terminal_output" => serde_json::Value::String(term_content.to_string()),
		_ => serde_json::Value::Null,
	}
}

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

		// Resolve the session id BEFORE entering any lock: hooks carry
		// `session_id` in their parsed args (pre_tool_call,
		// transform_tool_result, pre_llm_call, post_llm_call,
		// on_session_start); `transform_terminal_output` does NOT, so it
		// falls back to `debug::last_session()` like before. Empty ->
		// the default session key. Each hook then locks ONLY its own
		// session's state (per-session registry) with a bounded lock.
		let sid = parsed
			.get("session_id")
			.and_then(|v| v.as_str())
			.filter(|s| !s.is_empty())
			.map(str::to_string)
			.unwrap_or_else(|| {
				let last = crate::debug::last_session();
				if last.is_empty() { DEFAULT_SESSION.to_string() } else { last }
			});

		// `with_session` returns None when THIS session's lock is wedged by
		// an abandoned worker - fail open with the per-arm degrade value
		// (fail_open_hook) instead of blocking the gateway.
		let result:serde_json::Value = match with_session(&sid, |state| {
			match name.as_str() {
				// Accept both the canonical Hermes name and the legacy alias.
				"on_session_start" | "session_start" => {
					let r = aphrodite::session::on_session_start(state);
					// DEV-ONLY debug logger (crate::debug): session-init /
					// config-summary line, emitted once per process. Only when
					// Hermes actually threads a session_id here - the
					// last_session() fallback would target a STALE persisted
					// root from another process (e.g. the live parent session)
					// and append the init line to the WRONG debug file. The
					// pre_llm_call arm emits the init for the real session.
					if let Some(sid) = parsed.get("session_id").and_then(|v| v.as_str()) {
						log_session_init(state, sid);
					}
					r
				},
				"pre_tool_call" => {
					// DEV-ONLY debug logger (crate::debug): pre-tool line with
					// the tool name, args byte size, effective thresholds, and
					// the expected compression path (the transform arms log
					// the actual outcome).
					let call_tool = parsed.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
					let args_bytes = parsed.get("args").map(|a| a.to_string().len()).unwrap_or(0);
					let expected = expected_path(state, call_tool, &parsed);
					crate::debug::log_bridge(
						state,
						&crate::debug::last_session(),
						"pre_tool",
						&[
							("tool", call_tool.to_string()),
							("args_bytes", args_bytes.to_string()),
							("tool_threshold", state.tool_threshold.to_string()),
							("terminal_threshold", state.terminal_threshold.to_string()),
							("auto_expand_limit", state.auto_expand_limit.to_string()),
							("expected", expected),
						],
					);
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
					let sid = parsed.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
					// DEV-ONLY debug logger (crate::debug): post-tool outcome +
					// elapsed microseconds. Timing is captured only when a debug
					// channel is on for this session (quiet path: one load).
					let t0 = crate::debug::any_enabled(state, sid).then(std::time::Instant::now);
					let r = aphrodite::hooks::transform_tool_result_with_meta(
						state,
						tool_content,
						tool,
						classify.as_ref().map(|(c, t)| (c.as_str(), t.as_str())),
						&meta,
					);
					if let Some(t0) = t0 {
						let us = t0.elapsed().as_micros() as u64;
						aphrodite::debug::add_timing(aphrodite::debug::TimingCat::ToolTransform, us);
						post_tool_log(state, sid, &r, tool_content.len(), us);
					}
					// Per-session debug: the caller's session_id rides in the
					// hook kwargs - thread it so the flag resolves against the
					// session tree (root scoped flag wins, global fallback).
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
					// DEV-ONLY debug logger (crate::debug): post-tool outcome +
					// elapsed microseconds for the terminal path.
					let t0 = crate::debug::any_enabled(state, &sid).then(std::time::Instant::now);
					if let Some(t0) = t0 {
						let us = t0.elapsed().as_micros() as u64;
						aphrodite::debug::add_timing(aphrodite::debug::TimingCat::ToolTransform, us);
						post_tool_log(state, &sid, &r, term_content.len(), us);
					}
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
					// DEV-ONLY debug logger (crate::debug): pre-turn bridge line
					// (turn/session/request-size); the per-section sizes and the
					// cumulative metrics are logged inside build_turn_context.
					let sid = parsed.get("session_id").and_then(|v| v.as_str()).unwrap_or("");
					if !sid.is_empty() {
						crate::debug::log_bridge(
							state,
							sid,
							"pre_turn",
							&[
								("turn", state.turn_counter.to_string()),
								("session", sid.to_string()),
								("est_request_bytes", args.len().to_string()),
								("pending_markers", state.recent_markers.len().to_string()),
							],
						);
						log_session_init(state, sid);
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
		}) {
			Some(v) => v,
			None => fail_open_hook(&name, tool_content, term_content),
		};

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

	// ── DEV-ONLY debug logger (temporary instrumentation): the env-gated
	// logger must write timestamped structured lines to debug.<root> under
	// the runtime home, and a toggle written to that same file must stay
	// readable (first-line flag semantics coexist with the appended log
	// history). Self-cleaning: the probe file is removed in the test.
	#[test]
	fn test_debug_file_write_smoke() {
		let _g = test_guard();
		// Pin BOTH the core logger and the flag reader to the same scratch
		// home (the core crate compiles without cfg(test) as a dependency,
		// so isolation relies on the dev-only $APHRODITE_DEBUG_HOME
		// override, not on cfg(test)).
		const HOME_KEY:&str = "APHRODITE_DEBUG_HOME";
		let scratch = std::env::temp_dir().join("aphrodite-debug-smoke-home");
		let _ = std::fs::create_dir_all(&scratch);
		let backup = std::env::var(HOME_KEY).ok();
		unsafe { std::env::set_var(HOME_KEY, &scratch) };
		let root = "debug-smoke";
		let path = aphrodite::debug::log_path_for_root(root);
		aphrodite::debug::set_level(1);
		aphrodite::debug::note_root_session(root);
		let _ = std::fs::remove_file(&path);
		aphrodite::debug::log_at(1, "smoke", &[("k", "v".to_string())]);
		let content = std::fs::read_to_string(&path).unwrap_or_default();
		assert!(
			content.contains("event=smoke") && content.contains("k=v"),
			"debug file must carry the structured line, got: {content:?}"
		);
		assert!(content.starts_with('['), "line must be timestamped: {content:?}");
		// Toggle written to the same file (whole-file overwrite) must still
		// resolve as a flag: first-line/prefix semantics.
		std::fs::write(&path, "on\n").unwrap();
		assert!(crate::debug::enabled_for(root), "toggle must stay readable on line 1");
		// Regression: a legacy flag file may have NO trailing newline
		// ("on" bare), so a subsequent logger append merges into line 1
		// ("on[2026-...") - the flag must still read ON (prefix match).
		std::fs::write(&path, "on").unwrap();
		aphrodite::debug::log_at(1, "merged", &[]);
		assert!(
			crate::debug::enabled_for(root),
			"flag must survive an appended line merged into line 1"
		);
		let _ = std::fs::remove_file(&path);
		aphrodite::debug::set_level(0);
		unsafe { std::env::remove_var(HOME_KEY) };
		if let Some(v) = backup {
			unsafe { std::env::set_var(HOME_KEY, v) };
		}
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

	// ── Auto-expand: the hook resolved the stored content inline; the LLM
	// must receive the expanded string, never the marker. ──
	#[test]
	fn test_replacement_from_auto_expanded_returns_expanded() {
		let r = serde_json::json!({
			"status": "ok",
			"compressed": true,
			"auto_expanded": true,
			"hash": "dddd",
			"type": "terminal",
			"size": 11,
			"preview": "[terminal:11B] hi",
			"marker": "<<<CCR:dddd|terminal|11>>>",
			"expanded": "hello world"
		});
		let s = replacement_from(&r, "test-session");
		assert_eq!(
			s.as_str().unwrap(),
			"hello world",
			"auto-expanded payload must return the resolved content"
		);
	}

	#[test]
	fn test_replacement_from_auto_expanded_without_expanded_falls_back_to_marker() {
		// A payload flagging auto_expanded but carrying no usable `expanded`
		// string must fall through to the normal marker path, never produce
		// a null replacement that would erase the output.
		let r = serde_json::json!({
			"status": "ok",
			"compressed": true,
			"auto_expanded": true,
			"hash": "dddd",
			"type": "terminal",
			"size": 11,
			"preview": "[terminal:11B] hi",
			"marker": "<<<CCR:dddd|terminal|11>>>"
		});
		let s = replacement_from(&r, "test-session");
		assert_eq!(s.as_str().unwrap(), "<<<CCR:dddd|terminal|11>>>");
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
	// allocated and which freed it. This test round-trips fresh allocations (allocate → read back →
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
			let last = state.recent_markers.back().expect("hook must record a marker");
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

	// ── Config auto-reload: a fresh `Config` re-applied to the session
	// state must update config fields (thresholds) while PRESERVING the
	// runtime-owned directive selection (map + active list) - the exact
	// contract of the opt-in watcher (spawn_config_watcher). CCR stores
	// and telemetry are untouched by `apply_compression` itself. ──
	#[test]
	fn test_config_reload_preserves_directives_and_applies_thresholds() {
		let _g = crate::test_guard();

		let stamp = std::time::SystemTime::now()
			.duration_since(std::time::UNIX_EPOCH)
			.unwrap()
			.as_nanos();
		let path = std::env::temp_dir().join(format!("aphrodite-cfg-reload-{stamp}.toml"));
		std::fs::write(&path, "[compression]\nterminal_threshold = 2048\n").unwrap();
		let cfg = aphrodite::config_loader::Config::load_from(path.to_str().unwrap());
		let _ = std::fs::remove_file(&path);
		assert!(cfg.parse_failure.is_none());

		let mut state = AphroditeState::default();
		state.terminal_threshold = 1024;
		// Simulate a live session that swapped in a runtime directive set.
		state.directives.insert(
			"focus".into(),
			aphrodite::directives::Directive { name:"focus".into(), content:"stay targeted".into() },
		);
		state.active_directives = vec!["focus".into()];

		apply_config_reload(&cfg, &mut state);

		assert_eq!(state.terminal_threshold, 2048, "reload must apply new thresholds");
		assert!(
			state.directives.contains_key("focus"),
			"reload must preserve the runtime directive set"
		);
		assert_eq!(
			state.active_directives,
			vec!["focus".to_string()],
			"reload must preserve active directives"
		);
	}

	// ── Per-session state registry probe (gateway shared-plugin fix): two
	// different session_ids must resolve to DIFFERENT AphroditeState
	// instances. A compression stored under session A (via the production
	// transform_tool_result hook path) must NOT appear in session B's
	// catalog, nor in the default (tool) session - the pre-fix single global
	// state made every session see everything. ──
	#[test]
	fn probe_per_session_state_isolation() {
		let _g = crate::test_guard();
		// Reset the default (tool) session so its catalog count is
		// deterministic (other tests leave markers behind).
		with_shared(aphrodite::session::on_session_start);
		// DIRECT read (bypasses with_shared): the reset must have cleared the
		// real default state, not a throwaway fallback clone.
		let post_reset = {
			let s = session_state(DEFAULT_SESSION);
			let g = s.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
			g.recent_markers.len()
		};
		assert_eq!(
			post_reset, 0,
			"reset must clear the real default session state (not a fallback clone)"
		);

		// Compress under session "probe-a" through the hook path Hermes
		// actually fires (transform_tool_result with session_id in kwargs).
		let content = "probe-isolation-payload-xyz\n".repeat(400);
		let args = serde_json::json!({
			"tool_name": "terminal",
			"result": content,
			"session_id": "probe-a",
		})
		.to_string();
		let ptr = aphrodite_hermes_call_hook(
			CString::new("transform_tool_result").unwrap().as_ptr(),
			CString::new(args).unwrap().as_ptr(),
		);
		let hook_result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(ptr);
		// Compression happened: the hook returns the marker string.
		let marker:String = serde_json::from_str(&hook_result).expect("session A compress must return a marker");
		assert!(marker.starts_with("<<<CCR:"), "marker shape: {marker}");

		// Session B sees none of A's entries.
		let cat_b = with_session("probe-b", |s| serde_json::json!({"total": s.recent_markers.len()}))
			.expect("session B lock must be free");
		assert_eq!(cat_b["total"], 0, "session B must NOT see session A's compression");

		// Session A sees exactly its own entry.
		let cat_a = with_session("probe-a", |s| serde_json::json!({"total": s.recent_markers.len()}))
			.expect("session A lock must be free");
		assert_eq!(cat_a["total"], 1, "session A must see its own compression");

		// The default (tool) session - the with_shared resolution target in
		// tests, where last_session() is empty - must NOT see it either.
		let cat_def = with_shared(|s| serde_json::json!({"total": s.recent_markers.len()}));
		assert_eq!(
			cat_def["total"],
			0,
			"default session must NOT see session A's compression: last_session={:?}",
			crate::debug::last_session()
		);
	}

	// ── Wedged-lock degrade probe (gateway shared-plugin fix): simulate an
	// abandoned worker by HOLDING a session's state mutex, then call the
	// hook for THAT session. It must return the fail-open value within the
	// bounded-lock window (~100ms), never block toward Hermes' 50s hook
	// timeout (the pre-fix behavior that hung the gateway). ──
	#[test]
	fn probe_wedged_session_lock_degrades_fail_open() {
		let _g = crate::test_guard();
		// Simulate the abandoned worker: hold this session's lock.
		let state = session_state("probe-wedged");
		let _wedged_guard = state.lock().unwrap_or_else(std::sync::PoisonError::into_inner);

		// pre_tool_call for the wedged session -> null (no directive; the
		// tool proceeds normally).
		let t0 = std::time::Instant::now();
		let args = serde_json::json!({
			"tool_name": "terminal",
			"args": {"command": "cargo build"},
			"session_id": "probe-wedged",
		})
		.to_string();
		let ptr = aphrodite_hermes_call_hook(
			CString::new("pre_tool_call").unwrap().as_ptr(),
			CString::new(args).unwrap().as_ptr(),
		);
		let result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(ptr);
		let elapsed_ms = t0.elapsed().as_millis();
		assert_eq!(result, "null", "wedged pre_tool_call must fail open to null, got: {result}");
		assert!(
			elapsed_ms < 500,
			"wedged hook must return within ~200ms (bounded lock), took {elapsed_ms}ms"
		);

		// transform_tool_result for the wedged session -> the ORIGINAL
		// content string (never erase output).
		let content = "wedged-original-output-line\n".repeat(50);
		let args = serde_json::json!({
			"tool_name": "terminal",
			"result": content,
			"session_id": "probe-wedged",
		})
		.to_string();
		let t0 = std::time::Instant::now();
		let ptr = aphrodite_hermes_call_hook(
			CString::new("transform_tool_result").unwrap().as_ptr(),
			CString::new(args).unwrap().as_ptr(),
		);
		let hook_result = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
		aphrodite_hermes_free_string(ptr);
		let elapsed_ms = t0.elapsed().as_millis();
		let returned:String =
			serde_json::from_str(&hook_result).expect("wedged transform must return the original content string");
		assert_eq!(
			returned, content,
			"wedged transform_tool_result must return the ORIGINAL content, never erase it"
		);
		assert!(
			elapsed_ms < 500,
			"wedged transform must return within ~200ms (bounded lock), took {elapsed_ms}ms"
		);
	}
}
