//! Per-session debug toggle (flag files, NOT env).
//!
//! The gateway process env is fixed at launch - an env var exported
//! mid-session never reaches the running plugin, so the per-session switch
//! is a set of flag files under `~/.hermes/aphrodite/`:
//!
//!   debug.<root_session_id>   per-session-scoped (wins when present)
//!   debug                     global fallback (applies to every session)
//!
//! Content "on"/"1"/"debug" = enabled, anything else = off. Resolution walks
//! the session tree: Hermes threads `parent_session_id` through
//! `pre_llm_call` (turn_context.py:698-706) and every agent records its
//! parent, so `enabled_for(session)` walks session -> parent -> ... -> root
//! and checks the ROOT's scoped flag - meaning a toggle on the root session
//! applies to that session AND all its subagents/delegated tasks, but never
//! to other sessions. Both flag reads are mtime-cached (one stat() per call
//! when unchanged), mirroring the Python side's `_sync_debug` and the
//! dylib's own mtime hot-reload.
//!
//! When enabled, `debug_line` returns a `[aphrodite-debug ...]` line that
//! `replacement_from` prepends before the CCR marker, so the session sees
//! the debug output in the tool-result stream (before/after the markers).
//! Toggle from a session:
//!   echo on  > ~/.hermes/aphrodite/debug.<root-session-id>
//!   echo off > ~/.hermes/aphrodite/debug.<root-session-id>
use std::{
	collections::HashMap,
	path::{Path, PathBuf},
	sync::{Mutex, OnceLock},
};

/// session_id -> parent_session_id map, learned from `pre_llm_call` kwargs.
/// Bounded: a session records its parent exactly once; the map only grows
/// with the number of distinct sessions, which is small per process.
static SESSION_PARENTS:OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

/// The most recently seen session id, updated ONLY by `record_session` -
/// i.e. the `pre_llm_call` hook (turn_context.py threads session_id through
/// it). The `aphrodite_debug` tool has no session in its args: it toggles
/// whatever session the current LLM turn belongs to, so the flag must track
/// the turn's session, NOT whichever transform hook fired last (subagent
/// results could otherwise redirect the toggle to the wrong root).
static LAST_SESSION:OnceLock<Mutex<String>> = OnceLock::new();

/// flag-file path -> (mtime, enabled) cache; one entry per distinct flag.
static FLAG_CACHE:OnceLock<Mutex<HashMap<String, (f64, bool)>>> = OnceLock::new();

const FLAG_PREFIX:&str = "debug";

fn runtime_home() -> PathBuf {
	std::env::var_os("HOME")
		.map(PathBuf::from)
		.unwrap_or_else(std::env::temp_dir)
		.join(".hermes")
		.join("aphrodite")
}

/// Record session -> parent from a `pre_llm_call` invocation. Empty or
/// self-referential pairs are ignored (a root session has no parent).
pub(crate) fn record_session(session:&str, parent:&str) {
	if session.is_empty() {
		return;
	}
	{
		let mut last = LAST_SESSION
			.get_or_init(|| Mutex::new(String::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);
		*last = session.to_string();
	}
	// Persist across hot-reloads: a dylib reload wipes ALL Rust statics
	// (__init__.py:537), so the session id is mirrored to a tiny file that
	// `last_session()` reads back after a reload. Best-effort - a failed
	// write degrades to "no persistence", never an error.
	let _ = std::fs::write(runtime_home().join("session.current"), session);
	if parent.is_empty() || session == parent {
		return;
	}
	let mut map = SESSION_PARENTS
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	map.entry(session.to_string()).or_insert_with(|| parent.to_string());
}

/// Walk session -> parent -> ... -> root, root first. Depth-capped at 16 so
/// a malicious/cyclic chain cannot loop forever.
fn session_chain(session:&str) -> Vec<String> {
	let map = SESSION_PARENTS
		.get_or_init(|| Mutex::new(HashMap::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	let mut chain = vec![session.to_string()];
	let mut cur = session.to_string();
	for _ in 0..16 {
		match map.get(&cur) {
			Some(p) if p != &cur => {
				chain.push(p.clone());
				cur = p.clone();
			},
			_ => break,
		}
	}
	chain
}

fn flag_path_for(session:&str) -> PathBuf { runtime_home().join(format!("{}.{}", FLAG_PREFIX, session)) }

fn global_flag_path() -> PathBuf { runtime_home().join(FLAG_PREFIX) }

/// Mtime-cached read of one flag file; missing file = off.
fn flag_enabled(path:&Path) -> bool {
	let key = path.to_string_lossy().into_owned();
	let mtime = std::fs::metadata(path)
		.and_then(|m| m.modified())
		.ok()
		.map(|t| t.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0))
		.unwrap_or(0.0);
	let cache = FLAG_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
	let mut guard = cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	if let Some((cached_mtime, cached_on)) = guard.get(&key)
		&& *cached_mtime == mtime
	{
		return *cached_on;
	}
	let on = std::fs::read_to_string(path)
		.map(|s| s.trim().to_lowercase())
		.map(|s| matches!(s.as_str(), "1" | "on" | "true" | "debug"))
		.unwrap_or(false);
	guard.insert(key, (mtime, on));
	on
}

/// True when the debug flag is enabled for the given session.
///
/// Resolution: the session's ROOT scoped flag (`debug.<root>` - the
/// root-most session id in the chain) wins when it exists; subagents
/// resolve to the same root, so one toggle covers the whole tree, while
/// other sessions' roots differ and stay quiet. When NO scoped flag exists
/// the session is OFF - deliberately NOT global (a machine-wide `debug`
/// file only applies to calls with no session context, e.g. a non-Hermes
/// host or tests). This keeps the toggle strictly per session tree.
pub(crate) fn enabled_for(session:&str) -> bool {
	if session.is_empty() {
		// No session context: the plain `debug` flag applies (non-Hermes
		// hosts, standalone tool paths, tests without a session).
		return flag_enabled(&global_flag_path());
	}
	let chain = session_chain(session);
	// Root-most first: the top-level session's scoped flag governs the
	// whole tree. Walk root -> leaf and take the first scoped flag that
	// EXISTS (its value decides; absence falls through to OFF).
	for sid in chain.iter().rev() {
		let p = flag_path_for(sid);
		if p.exists() {
			return flag_enabled(&p);
		}
	}
	false
}

/// Resolve the most recently seen session's ROOT id.
#[cfg(debug_assertions)]
fn current_root() -> String {
	let last = LAST_SESSION
		.get_or_init(|| Mutex::new(String::new()))
		.lock()
		.unwrap_or_else(std::sync::PoisonError::into_inner);
	if last.is_empty() {
		return String::new();
	}
	session_chain(&last).last().cloned().unwrap_or_else(|| last.clone())
}

/// The most recently seen session id (the current LLM turn's session, set by
/// `pre_llm_call`). Fallback for hooks Hermes does NOT thread session_id
/// through - `transform_terminal_output` passes only command/output/
/// returncode/task_id/env_type (terminal_tool_result.py:144), so the terminal
/// arm falls back to this instead of losing the session scope.
///
/// Persists across dylib hot-reloads: `record_session` also writes the id to
/// a tiny file in the runtime home, and a reload (which wipes all Rust
/// statics - __init__.py:537) falls back to reading that file. Without this,
/// a mid-turn rebuild would leave terminal output with no session scope.
pub(crate) fn last_session() -> String {
	{
		let last = LAST_SESSION
			.get_or_init(|| Mutex::new(String::new()))
			.lock()
			.unwrap_or_else(std::sync::PoisonError::into_inner);
		if !last.is_empty() {
			return last.clone();
		}
	}
	// Reload fallback: read the persisted session id from the runtime home.
	std::fs::read_to_string(runtime_home().join("session.current"))
		.ok()
		.map(|s| s.trim().to_string())
		.unwrap_or_default()
}

/// Set (or clear) the debug flag for the CURRENT session tree - the tool
/// entry point for `aphrodite_debug`. Returns the resolved root session id
/// and the flag path written, so the caller can report both.
///
/// Uses the persisted session id (falling back to `session.current` across
/// hot-reloads, same as `last_session`): a reload wipes LAST_SESSION, and
/// without the file fallback the toggle would report "no session context"
/// until the next `pre_llm_call`.
///
/// Dev-only: the `aphrodite_debug` tool that calls this is gated behind
/// debug_assertions (release dylibs register exactly the 13 production
/// tools), so this entry point is dead code in release builds.
#[cfg(debug_assertions)]
pub(crate) fn set_enabled_current(on:bool) -> Result<(String, String), String> {
	let root = current_root();
	let root = if root.is_empty() { last_session() } else { root };
	if root.is_empty() {
		return Err("no session context yet - hooks have not fired in this process".to_string());
	}
	let flag = flag_path_for(&root);
	std::fs::write(&flag, if on { "on" } else { "off" }).map_err(|e| format!("write {}: {}", flag.display(), e))?;
	// Invalidate the mtime cache so the next read picks up the new value.
	let key = flag.to_string_lossy().into_owned();
	if let Some(cache) = FLAG_CACHE.get() {
		let mut guard = cache.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		guard.remove(&key);
	}
	Ok((root, flag.to_string_lossy().into_owned()))
}

/// Build a `[aphrodite-debug ...]` prefix line from a compression result
/// when the flag is on for `session`; `None` when off (no allocation on the
/// quiet path).
pub(crate) fn debug_line(r:&serde_json::Value, session:&str) -> Option<String> {
	if !enabled_for(session) {
		return None;
	}
	let status = r.get("status").and_then(|v| v.as_str()).unwrap_or("?");
	let compressed = r.get("compressed").and_then(|v| v.as_bool()).unwrap_or(false);
	let ccr_type = r.get("type").and_then(|v| v.as_str()).unwrap_or("?");
	let size = r.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
	let hash = r.get("hash").and_then(|v| v.as_str()).unwrap_or("");
	let reason = r.get("reason").and_then(|v| v.as_str()).unwrap_or("");
	Some(format!(
		"[aphrodite-debug: session={} status={} compressed={} type={} size={} hash={} reason={}]",
		session, status, compressed, ccr_type, size, hash, reason
	))
}
