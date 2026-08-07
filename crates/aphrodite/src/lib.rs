//! aphrodite: core compression engine exposed as both an rlib (for the
//! `aphrodite` binary and the `aphrodite-hermes` bridge) and a C ABI cdylib.
//!
//! The C ABI is agent-agnostic and handle-based: `aphrodite_init` allocates a
//! session handle, the `aphrodite_*` functions operate on it (classify,
//! compress, retrieve, transform, terminal, session_start, catalog, stats,
//! reload, config_get/set, search, dispatch, resolve, stage2, struct_extract),
//! and `aphrodite_destroy` frees it. The Hermes plugin uses the higher-level,
//! process-global `aphrodite_hermes_*` ABI in the `aphrodite-hermes` crate.

// Every `pub extern "C" fn` here takes a raw pointer without being marked
// `unsafe fn` itself (the null/UTF-8 checks happen inside `cstr()`, not at
// the signature level), which is exactly what this lint flags. Fixing it
// properly means marking each function `unsafe extern "C" fn` and auditing
// every caller across this crate, `aphrodite-hermes`, and the Python ctypes
// bindings for the safety-contract change - that's the FFI/C-ABI boundary
// report's job (.plans/03-ffi-c-abi.md), not a side effect of wiring up a
// CI clippy gate. Scoped here so the gate is honest today; remove this once
// report 03 lands.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

pub mod catalog;
pub mod config_loader;
pub mod directives;
pub mod flow;
pub mod hooks;
pub mod marker;
#[cfg(feature = "navigation")]
pub mod navigate;
pub mod poll_worker;
pub mod prefetch;
pub mod resolve;
pub mod session;
pub mod stage2;
pub mod state;
pub mod struct_extract;

// Proxy modules (used by main.rs binary) - gated behind the `proxy`
// feature (report 01-T2) so the cdylib built by `aphrodite-hermes` with
// `default-features = false` doesn't link axum/tokio/reqwest into the
// Hermes dylib unnecessarily.
// center.rs removed (report 01-T6): zero callers, dead machinery
#[cfg(feature = "proxy")]
pub mod config;
#[cfg(feature = "proxy")]
pub mod proxy;
#[cfg(feature = "proxy")]
pub mod retrieve;
#[cfg(feature = "proxy")]
pub mod setup;

use std::{
	collections::HashMap,
	ffi::{CStr, CString},
	os::raw::c_char,
	sync::{Arc, Mutex},
};

use headroom_core::transforms;
use state::AphroditeState;

// ── Hardened primitives ──────────────────────────────────────────────

const MAX_CONTENT: usize = 16 * 1024 * 1024; // 16MB cap

/// Process-global handle table. `None` until the first handle is allocated,
/// so `Mutex::new` can stay `const` without requiring a heap alloc at
/// startup; `handles()` lazily initializes it to `Some` on first use.
///
/// Each session's state lives behind its own `Arc<Mutex<..>>` (F4/T6, report
/// 06) rather than directly in the map: the outer `HANDLES` mutex is only
/// ever held long enough to look up and clone that `Arc`, never across a
/// hook/tool body. Previously one global lock serialized every session's
/// every call - two different agent sessions could not classify/compress
/// concurrently at all, even though they share nothing.
static HANDLES: Mutex<Option<HashMap<usize, Arc<Mutex<AphroditeState>>>>> = Mutex::new(None);
/// Next handle ID to hand out from `alloc_handle`. Wraps on overflow rather
/// than panicking - see the `wrapping_add` call there.
static NEXT_ID: Mutex<usize> = Mutex::new(1);

fn handles() -> std::sync::MutexGuard<'static, Option<HashMap<usize, Arc<Mutex<AphroditeState>>>>> {
	let mut g = HANDLES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	if g.is_none() {
		*g = Some(HashMap::new());
	}
	g
}

/// Look up a session's handle and clone its `Arc` out from under the map
/// lock, which is dropped before returning - callers lock the per-session
/// mutex separately so the map lock is never held across state access.
fn get_handle(hid: usize) -> Option<Arc<Mutex<AphroditeState>>> {
	handles().as_ref().and_then(|m| m.get(&hid)).cloned()
}

fn alloc_handle(state: AphroditeState) -> usize {
	let mut id = NEXT_ID.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	let hid = *id;
	*id = id.wrapping_add(1); // overflow-safe
	handles().as_mut().unwrap().insert(hid, Arc::new(Mutex::new(state)));
	hid
}

/// Torn-state contract (report 06 F12): `f` runs under `catch_unwind`, so a
/// panicking hook returns an error instead of aborting the process, but any
/// mutation `f` already made to `state` before panicking is NOT rolled back.
/// A panic between two steps of a multi-step mutation (e.g. `retain` then
/// `push_front` in `inline_store_put`) leaves whatever partial state existed
/// at the panic point for the next caller to observe. This crate's mutation
/// sequences are ordered so an interruption degrades to "entry missing"
/// rather than "duplicate entry" - keep that ordering if you touch them.
fn with_state<T>(hid: usize, f: impl FnOnce(&mut AphroditeState) -> T) -> Result<T, String> {
	let session = match get_handle(hid) {
		Some(s) => s,
		None => return Err(format!("invalid handle: {}", hid)),
	};
	let mut state = session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut state)))
		.map_err(|_| "internal error: hook panicked".to_string())
}

fn to_json_error(msg: &str) -> *mut c_char {
	let json = serde_json::json!({"error": msg}).to_string();
	CString::new(json).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// Run `f` under `catch_unwind`, converting a panic into an error-JSON
/// `*mut c_char` instead of unwinding across the `extern "C"` boundary
/// (which triggers the Rust runtime's forced process abort). `with_state`
/// already gives stateful fns this guarantee; this covers every extern fn
/// that doesn't go through `with_state` (classify, retrieve, filter_lines,
/// preview, stage2, struct_extract, init, catalog, stats, search, directive,
/// config_get) - previously several of these could abort the host process
/// on a panic (e.g. the byte-slicing panics in struct_extract.rs/marker.rs,
/// now fixed separately, but any future panic in these paths would have the
/// same effect).
///
/// Rule: every new `pub extern "C" fn` added to this file must either route
/// through `with_state` (which guards internally) or wrap its body in
/// `guarded(AssertUnwindSafe(...))` directly - there is no third option that
/// stays panic-safe across the FFI boundary.
fn guarded(f: impl FnOnce() -> *mut c_char + std::panic::UnwindSafe) -> *mut c_char {
	std::panic::catch_unwind(f).unwrap_or_else(|_| to_json_error("internal error: panicked"))
}

fn to_json_ok(v: &serde_json::Value) -> *mut c_char {
	CString::new(v.to_string())
		.map(|c| c.into_raw())
		.unwrap_or(std::ptr::null_mut())
}

unsafe fn cstr(ptr: *const c_char) -> Option<String> {
	if ptr.is_null() {
		return None;
	}
	Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

fn check_content(content: &str) -> Result<(), &'static str> {
	if content.is_empty() {
		return Err("empty content");
	}
	if content.len() > MAX_CONTENT {
		return Err("content exceeds 16MB limit");
	}
	if content.contains('\0') {
		return Err("content contains null bytes");
	}
	Ok(())
}

// ── C ABI ────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn aphrodite_version() -> *mut c_char {
	CString::new(env!("CARGO_PKG_VERSION")).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn aphrodite_free_string(s: *mut c_char) {
	if !s.is_null() {
		unsafe {
			let _ = CString::from_raw(s);
		}
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_hooks() -> *mut c_char {
	// `on_session_start`, not `session_start`: Hermes's VALID_HOOKS table
	// requires the `on_` prefix - registering the bare name silently no-ops
	// in Hermes (the bridge crate already returns the correct name; this
	// discovery fn used to hand out the broken one). `aphrodite_dispatch`/
	// `aphrodite_call_hook` accept both names for compatibility.
	CString::new(
		serde_json::json!([
			"on_session_start",
			"pre_tool_call",
			"transform_tool_result",
			"transform_terminal_output",
			"pre_llm_call",
			"post_llm_call"
		])
		.to_string(),
	)
	.unwrap()
	.into_raw()
}

#[no_mangle]
pub extern "C" fn aphrodite_init(config_path: *const c_char) -> *mut c_char {
	let path = unsafe { cstr(config_path) }.unwrap_or_default();
	guarded(std::panic::AssertUnwindSafe(move || {
		let mut state = AphroditeState::default();
		// 01-F4/F9: delegate to `config_loader::Config` instead of hand-parsing
		// four `[compression]` keys here - this hand-rolled copy had already
		// drifted from `apply_compression`'s own key names (e.g. `tool_threshold`
		// vs `tool_threshold_token`) and never honored env var overrides, unlike
		// every other init path in the crate.
		if !path.is_empty() {
			crate::config_loader::Config::load_from(&path).apply_compression(&mut state);
		}
		CString::new(alloc_handle(state).to_string()).unwrap().into_raw()
	}))
}

#[no_mangle]
pub extern "C" fn aphrodite_destroy(handle: *const c_char) {
	if let Ok(hid) = unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		handles().as_mut().and_then(|m| m.remove(&hid));
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_classify(content: *const c_char) -> *mut c_char {
	let c = match unsafe { cstr(content) } {
		Some(s) => s,
		None => return to_json_error("null content"),
	};
	guarded(std::panic::AssertUnwindSafe(move || {
		if let Err(e) = check_content(&c) {
			return to_json_error(e);
		}
		let ct = transforms::content_detector::detect_content_type(&c).content_type;
		to_json_ok(&serde_json::json!({"type":ct.as_str(),"lines":c.lines().count(),"bytes":c.len()}))
	}))
}

/// Deprecated (report 01-T5): stateless, creates a throwaway AphroditeState
/// per call. Use `aphrodite_init` + `aphrodite_dispatch` (stateful, handle-based)
/// instead. This stub keeps the ABI symbol alive for any legacy consumer that
/// hasn't migrated yet.
#[no_mangle]
pub extern "C" fn aphrodite_call_hook(_hook: *const c_char, _args: *const c_char) -> *mut c_char {
	to_json_error("aphrodite_call_hook is stateless and deprecated; use aphrodite_init + aphrodite_dispatch")
}

// ── Stateful operations ──────────────────────────────────────────────

macro_rules! stateful {
    ($name:ident, |$s:ident, $($arg:ident : $ty:ty),*| $body:expr) => {
        #[no_mangle] pub extern "C" fn $name(handle: *const c_char, $($arg: *const c_char),*) -> *mut c_char {
            let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() { Ok(id) => id, Err(_) => return to_json_error("invalid handle") };
            $(let $arg = unsafe { cstr($arg) }.unwrap_or_default();)*
            match with_state(hid, |$s| $body) {
                Ok(v) => to_json_ok(&v),
                Err(e) => to_json_error(&e),
            }
        }
    };
}

stateful!(aphrodite_compress, |s, content: *const c_char, hint: *const c_char| {
	if content.is_empty() {
		return serde_json::json!({"error":"empty"});
	}
	if content.len() > MAX_CONTENT {
		return serde_json::json!({"error":"content exceeds 16MB limit"});
	}
	let ct = transforms::content_detector::detect_content_type(&content).content_type;
	let t = if hint.is_empty() || hint == "text" {
		ct.as_str().to_string()
	} else {
		hint.to_string()
	};
	let hash = headroom_core::ccr::compute_key(content.as_bytes());
	s.inline_store_put(hash.clone(), content.to_string());
	let preview = crate::build_preview(&t, &content);
	let marker = marker::ccr_marker(&hash, &t, content.len(), &preview, None, None, None);
	s.record_marker(state::MarkerEntry {
		hash: hash.clone(),
		ccr_type: t.clone(),
		size: content.len(),
		preview: preview.clone(),
		turn: s.turn_counter,
		center: None,
		meta: None,
	});
	serde_json::json!({"hash":hash,"type":t,"size":content.len(),"preview":preview,"marker":marker})
});

// aphrodite_retrieve is a manual override below - returns raw content, not JSON

// Override: retrieve returns raw content, not JSON-wrapped
#[no_mangle]
pub extern "C" fn aphrodite_retrieve(handle: *const c_char, hash: *const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let hash = unsafe { cstr(hash) }.unwrap_or_default();
	guarded(std::panic::AssertUnwindSafe(move || {
		let session = match get_handle(hid) {
			Some(s) => s,
			None => return to_json_error(&format!("invalid handle: {}", hid)),
		};
		let mut s = session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
		match s.inline_store_get(&hash) {
			Some(content) => CString::new(content.replace('\0', "")).unwrap().into_raw(),
			None => to_json_error(&format!("hash not found: {}", hash)),
		}
	}))
}

stateful!(aphrodite_transform, |s, content: *const c_char, tool: *const c_char| {
	if content.len() > MAX_CONTENT {
		return serde_json::json!({"error":"content exceeds 16MB limit"});
	}
	hooks::transform_tool_result(s, &content, &tool)
});

stateful!(aphrodite_terminal, |s, content: *const c_char| {
	if content.len() > MAX_CONTENT {
		return serde_json::json!({"error":"content exceeds 16MB limit"});
	}
	hooks::transform_terminal_output(s, &content)
});

#[no_mangle]
pub extern "C" fn aphrodite_session_start(handle: *const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	match with_state(hid, session::on_session_start) {
		Ok(v) => to_json_ok(&v),
		Err(e) => to_json_error(&e),
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_catalog(handle: *const c_char, mode: *const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let m = unsafe { cstr(mode) }.unwrap_or_default();
	guarded(std::panic::AssertUnwindSafe(move || match get_handle(hid) {
		Some(session) => {
			let s = session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
			to_json_ok(&crate::catalog::build_catalog(&s, &m))
		},
		None => to_json_error(&format!("invalid handle: {}", hid)),
	}))
}

#[no_mangle]
pub extern "C" fn aphrodite_stats(handle: *const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	guarded(std::panic::AssertUnwindSafe(move || match get_handle(hid) {
		Some(session) => {
			let s = session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
			to_json_ok(&serde_json::json!({
				"version":env!("CARGO_PKG_VERSION"),"inline_entries":s.inline_store.len(),
				"markers":s.recent_markers.len(),"turn":s.turn_counter,
				"engine_enabled":s.context_engine_enabled,"threshold_pct":s.engine_threshold_pct,
				"tool_threshold":s.tool_threshold,"terminal_threshold":s.terminal_threshold,
			}))
		},
		None => to_json_error(&format!("invalid handle: {}", hid)),
	}))
}

#[no_mangle]
pub extern "C" fn aphrodite_reload(handle: *const c_char, path: *const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let p = unsafe { cstr(path) }.unwrap_or_default();
	// Read the config file BEFORE taking the HANDLES lock (F9): this fn used
	// to do the read inside `with_state`'s closure, holding the *global*
	// handle-map mutex across disk I/O - on a slow/cold mount, one handle's
	// reload blocks every other handle's every call, and it's a trap for a
	// future `f` that calls back into another `aphrodite_*` fn (non-reentrant
	// Mutex -> instant deadlock).
	let context_engine_enabled = if !p.is_empty() {
		std::fs::read_to_string(p.as_str())
			.ok()
			.and_then(|t| t.parse::<toml::Table>().ok())
			.and_then(|tbl| tbl.get("compression").and_then(|v| v.as_table()).cloned())
			.and_then(|c| c.get("context_engine").and_then(|v| v.as_bool()))
	} else {
		None
	};
	match with_state(hid, |s| {
		if let Some(v) = context_engine_enabled {
			s.context_engine_enabled = v;
		}
		serde_json::json!({"status":"ok"})
	}) {
		Ok(v) => to_json_ok(&v),
		Err(e) => to_json_error(&e),
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_search(handle: *const c_char, query: *const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let q = unsafe { cstr(query) }.unwrap_or_default().to_lowercase();
	guarded(std::panic::AssertUnwindSafe(move || match get_handle(hid) {
		Some(session) => {
			let s = session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
			let results: Vec<serde_json::Value> = s
				.recent_markers
				.iter()
				.filter(|m| m.preview.to_lowercase().contains(&q) || m.ccr_type.to_lowercase().contains(&q))
				.take(20)
				.map(|m| serde_json::json!({"hash":&m.hash,"type":m.ccr_type,"size":m.size,"preview":m.preview}))
				.collect();
			to_json_ok(&serde_json::json!({"total":results.len(),"results":results}))
		},
		None => to_json_error(&format!("invalid handle: {}", hid)),
	}))
}

#[no_mangle]
pub extern "C" fn aphrodite_directive(
	handle: *const c_char,
	action: *const c_char,
	name: *const c_char,
) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let act = unsafe { cstr(action) }.unwrap_or_default();
	let nm = unsafe { cstr(name) }.unwrap_or_default();
	guarded(std::panic::AssertUnwindSafe(move || match get_handle(hid) {
		Some(session) => {
			let mut s = session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
			to_json_ok(&directives::handle_action(&mut s, &act, &nm))
		},
		None => to_json_error(&format!("invalid handle: {}", hid)),
	}))
}

#[no_mangle]
pub extern "C" fn aphrodite_config_get(handle: *const c_char, key: *const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let k = unsafe { cstr(key) }.unwrap_or_default();
	guarded(std::panic::AssertUnwindSafe(move || match get_handle(hid) {
		Some(session) => {
			let s = session.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
			let v = match k.as_ref() {
				"model" => s.model.clone(),
				"api_url" => s.api_url.clone(),
				"engine_threshold_pct" => s.engine_threshold_pct.to_string(),
				"tool_threshold" => s.tool_threshold.to_string(),
				"context_engine_enabled" => s.context_engine_enabled.to_string(),
				_ => return to_json_error(&format!("unknown key: {}", k)),
			};
			CString::new(v.replace('\0', "")).unwrap().into_raw()
		},
		None => to_json_error(&format!("invalid handle: {}", hid)),
	}))
}

#[no_mangle]
pub extern "C" fn aphrodite_config_set(handle: *const c_char, key: *const c_char, value: *const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let k = unsafe { cstr(key) }.unwrap_or_default();
	let v = unsafe { cstr(value) }.unwrap_or_default();
	match with_state(hid, |s| {
		match k.as_ref() {
			"model" => s.model = v.to_string(),
			"engine_threshold_pct" => {
				if let Ok(n) = v.parse() {
					s.engine_threshold_pct = n;
				}
			},
			"tool_threshold" => {
				if let Ok(n) = v.parse() {
					s.tool_threshold = n;
				}
			},
			"context_engine_enabled" => s.context_engine_enabled = v == "true" || v == "1",
			_ => {},
		}
		serde_json::json!({"status":"ok"})
	}) {
		Ok(v) => to_json_ok(&v),
		Err(e) => to_json_error(&e),
	}
}

// ── Preview builder ──────────────────────────────────────────────────
//
// Moved to `crate::preview` (report 01-T11); re-exported here so
// `aphrodite-hermes`'s `use aphrodite::{build_preview, detect_type}` stays
// source-compatible.
pub mod preview;
pub use preview::{build_preview, detect_type};

// ── Universal dispatch: all Python hooks route through here ──

/// Universal hook dispatcher. Python calls this for every hook handler.
/// Returns JSON-wrapped result or raw string if content-only.
#[no_mangle]
pub extern "C" fn aphrodite_dispatch(
	handle: *const c_char,
	hook_name: *const c_char,
	args_json: *const c_char,
) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let name = unsafe { cstr(hook_name) }.unwrap_or_default();
	let args_str = unsafe { cstr(args_json) }.unwrap_or_default();

	let args: serde_json::Value = match serde_json::from_str(&args_str) {
		Ok(v) => v,
		Err(e) => return to_json_error(&format!("invalid args: {}", e)),
	};

	let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
	if content.len() > MAX_CONTENT {
		return to_json_error("content exceeds 16MB limit");
	}
	let tool = args.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");

	// F9: read prefetch files from disk BEFORE taking the HANDLES lock, not
	// inside `with_state`'s closure - up to 10MB per file was previously
	// read while holding the *global* handle-map mutex, serializing every
	// other session's every call behind this one's (possibly slow/cold-mount)
	// disk reads.
	// RefCell, not a plain local: `with_state`'s closure below still borrows
	// `args`/`content`/`tool` for the other match arms, so it can't also
	// `move` this out of an owned `Option` - a `RefCell` lets the "prefetch"
	// arm take it through a shared capture instead.
	let prefetch_outcomes = std::cell::RefCell::new(if name == "prefetch" {
		let paths = args.get("paths").and_then(|v| v.as_array()).cloned().unwrap_or_default();
		let path_strings: Vec<String> = paths.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
		Some(crate::prefetch::read_paths(&path_strings))
	} else {
		None
	});

	let result = match with_state(hid, |s| {
		match name.as_ref() {
			"on_session_start" | "session_start" => hooks::on_session_start(s),
			"transform_tool_result" => hooks::transform_tool_result(s, content, tool),
			"transform_terminal_output" => hooks::transform_terminal_output(s, content),
			"pre_llm_call" => hooks::pre_llm_call(s),
			"post_llm_call" => hooks::post_llm_call(s),
			"catalog" => {
				let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("full");
				crate::catalog::build_catalog(s, mode)
			},
			"stats" => {
				serde_json::json!({
					"version": env!("CARGO_PKG_VERSION"),
					"inline_entries": s.inline_store.len(),
					"markers": s.recent_markers.len(),
					"turn": s.turn_counter,
					"engine_enabled": s.context_engine_enabled,
				})
			},
			"directive" => {
				let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("list");
				let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("");
				directives::handle_action(s, action, name)
			},
			"search" => {
				let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
				let type_filter = args.get("type").and_then(|v| v.as_str());
				let results: Vec<serde_json::Value> = s
					.recent_markers
					.iter()
					.filter(|m| {
						let matches_query = query.is_empty()
							|| m.preview.to_lowercase().contains(&query)
							|| m.ccr_type.to_lowercase().contains(&query);
						let matches_type = type_filter.is_none_or(|t| m.ccr_type == t);
						matches_query && matches_type
					})
					.take(20)
					.map(|m| serde_json::json!({"hash":&m.hash,"type":m.ccr_type,"size":m.size,"preview":m.preview}))
					.collect();
				serde_json::json!({"total":results.len(),"results":results})
			},
			"diff" => {
				let turns: Vec<serde_json::Value> = s
					.conv_index
					.iter()
					.map(
						|(turn, (hash, summary, size))| serde_json::json!({"turn":turn,"hash":hash,"summary":summary,"size":size}),
					)
					.collect();
				serde_json::json!({"turns":turns,"total":turns.len()})
			},
			"files" => {
				let files: Vec<serde_json::Value> = s
					.referenced_files
					.iter()
					.map(|(path, tool)| serde_json::json!({"path":path,"tool":tool}))
					.collect();
				serde_json::json!({"files":files,"total":files.len()})
			},
			"classify" => {
				let ct = headroom_core::transforms::content_detector::detect_content_type(content).content_type;
				serde_json::json!({"type":ct.as_str(),"lines":content.lines().count(),"bytes":content.len()})
			},
			"prefetch" => {
				// `prefetch_outcomes` is always `Some` here because
				// `name == "prefetch"` is exactly the condition that set it
				// above, before this closure runs.
				let outcomes = prefetch_outcomes
					.borrow_mut()
					.take()
					.expect("prefetch_outcomes set when name == \"prefetch\"");
				crate::prefetch::insert_outcomes(s, outcomes)
			},
			_ => serde_json::json!({"error": format!("unknown hook: {}", name)}),
		}
	}) {
		Ok(v) => to_json_ok(&v),
		Err(e) => to_json_error(&e),
	};

	result
}

/// Filter lines by query - port of _resolve.py _filter_lines
#[no_mangle]
pub extern "C" fn aphrodite_filter_lines(content: *const c_char, query: *const c_char) -> *mut c_char {
	let c = match unsafe { cstr(content) } {
		Some(s) => s,
		None => return to_json_error("null content"),
	};
	let q = unsafe { cstr(query) }.unwrap_or_default();
	guarded(std::panic::AssertUnwindSafe(move || {
		let filtered = crate::resolve::filter_lines(&c, &q);
		CString::new(filtered.replace('\0', "")).unwrap().into_raw()
	}))
}

/// Resolve hash with full recursive expansion - port of _resolve.py
#[no_mangle]
pub extern "C" fn aphrodite_resolve(handle: *const c_char, hash: *const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let h = unsafe { cstr(hash) }.unwrap_or_default();
	match with_state(hid, |s| match crate::resolve::expand(s, &h) {
		Some(content) => serde_json::json!({"found":true,"content":content}),
		None => serde_json::json!({"found":false}),
	}) {
		Ok(v) => to_json_ok(&v),
		Err(e) => to_json_error(&e),
	}
}

/// Generate preview for content - port of _marker/preview.py
#[no_mangle]
pub extern "C" fn aphrodite_preview(content: *const c_char, ccr_type: *const c_char) -> *mut c_char {
	let c = match unsafe { cstr(content) } {
		Some(s) => s,
		None => return to_json_error("null content"),
	};
	let t = unsafe { cstr(ccr_type) }.unwrap_or_default();
	guarded(std::panic::AssertUnwindSafe(move || {
		let preview = crate::build_preview(&t, &c);
		CString::new(preview.replace('\0', "")).unwrap().into_raw()
	}))
}

/// Stage 2 semantic reduction - port of _stage2.py
#[no_mangle]
pub extern "C" fn aphrodite_stage2(content: *const c_char, ccr_type: *const c_char) -> *mut c_char {
	let c = match unsafe { cstr(content) } {
		Some(s) => s,
		None => return to_json_error("null content"),
	};
	let t = unsafe { cstr(ccr_type) }.unwrap_or_default();
	guarded(std::panic::AssertUnwindSafe(move || {
		match crate::stage2::compress_stage2(&c, &t) {
			Some(reduced) => CString::new(reduced.replace('\0', "")).unwrap().into_raw(),
			None => to_json_error("no reduction possible"),
		}
	}))
}

/// Code structure extraction - port of _core/struct.py
#[no_mangle]
pub extern "C" fn aphrodite_struct_extract(content: *const c_char, language: *const c_char) -> *mut c_char {
	let c = match unsafe { cstr(content) } {
		Some(s) => s,
		None => return to_json_error("null content"),
	};
	let lang = unsafe { cstr(language) }.unwrap_or_default();
	guarded(std::panic::AssertUnwindSafe(move || {
		let result = crate::struct_extract::extract_code_structure(&c, &lang);
		to_json_ok(&serde_json::json!(result))
	}))
}

#[cfg(test)]
mod ffi_tests {
	use super::*;

	// ── T9 (F10): build_preview's "terminal" arm ──────────────────
	// `hooks::transform_terminal_output` overrides the classified type to
	// "terminal" for exit-code/error-shaped output, but `build_preview` had
	// no matching arm, so the preview silently fell through to the generic
	// `_` branch (a bare line/byte count with no exit-code context).
	#[test]
	fn test_build_preview_terminal_surfaces_exit_code() {
		let preview = build_preview("terminal", "running tests\nall good\nexit code: 1\n");
		assert!(preview.starts_with("[terminal:"));
		assert!(
			preview.contains("exit code: 1"),
			"preview should surface the exit code line: {preview}"
		);
	}

	#[test]
	fn test_build_preview_terminal_falls_back_to_last_line() {
		let preview = build_preview("terminal", "line one\nline two\nlast line here\n");
		assert!(preview.starts_with("[terminal:"));
		assert!(
			preview.contains("last line here"),
			"preview should fall back to the last non-empty line: {preview}"
		);
	}

	fn cs(s: &str) -> CString {
		CString::new(s).unwrap()
	}

	unsafe fn take(ptr: *mut c_char) -> String {
		assert!(!ptr.is_null(), "expected non-null C string");
		let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
		aphrodite_free_string(ptr);
		s
	}

	// ── T6 (F4, report 06): per-handle locking, not one global mutex ──
	// Previously `HANDLES` held every session's `AphroditeState` directly
	// under one `Mutex`, so a long-running call on one session (e.g. a slow
	// hook) serialized every OTHER session's calls behind it too, even
	// though sessions share no state. Verifies handle 2's call completes
	// quickly while handle 1's call is still sleeping inside its own lock.
	#[test]
	fn with_state_does_not_serialize_across_different_handles() {
		let h1: usize = unsafe { take(aphrodite_init(std::ptr::null())) }.parse().unwrap();
		let h2: usize = unsafe { take(aphrodite_init(std::ptr::null())) }.parse().unwrap();

		let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
		let b1 = barrier.clone();
		let t1 = std::thread::spawn(move || {
			with_state(h1, |_s| {
				b1.wait();
				std::thread::sleep(std::time::Duration::from_millis(300));
			})
			.unwrap();
		});

		barrier.wait();
		let start = std::time::Instant::now();
		with_state(h2, |_s| {}).unwrap();
		let elapsed = start.elapsed();

		t1.join().unwrap();
		handles().as_mut().and_then(|m| m.remove(&h1));
		handles().as_mut().and_then(|m| m.remove(&h2));

		assert!(
			elapsed < std::time::Duration::from_millis(150),
			"handle 2's call should not block behind handle 1's long-held lock, took {elapsed:?}"
		);
	}

	#[test]
	fn init_with_null_config_yields_parseable_handle() {
		let h = unsafe { take(aphrodite_init(std::ptr::null())) };
		assert!(h.parse::<usize>().is_ok(), "handle {:?} should parse as usize", h);
		aphrodite_destroy(cs(&h).as_ptr());
	}

	#[test]
	fn compress_retrieve_destroy_roundtrip() {
		let h = unsafe { take(aphrodite_init(std::ptr::null())) };
		let handle = cs(&h);

		let compress_json = unsafe {
			take(aphrodite_compress(
				handle.as_ptr(),
				cs("fn main() {}").as_ptr(),
				cs("text").as_ptr(),
			))
		};
		let v: serde_json::Value = serde_json::from_str(&compress_json).unwrap();
		let hash = v["hash"].as_str().unwrap().to_string();
		assert!(!hash.is_empty());

		let retrieved = unsafe { take(aphrodite_retrieve(handle.as_ptr(), cs(&hash).as_ptr())) };
		assert_eq!(retrieved, "fn main() {}");

		aphrodite_destroy(handle.as_ptr());
		// Handle is gone: any further stateful call reports invalid handle, no crash.
		let after = unsafe { take(aphrodite_stats(handle.as_ptr())) };
		assert!(after.contains("invalid handle"));
	}

	#[test]
	fn retrieve_unknown_hash_returns_error_json() {
		let h = unsafe { take(aphrodite_init(std::ptr::null())) };
		let handle = cs(&h);
		let out = unsafe { take(aphrodite_retrieve(handle.as_ptr(), cs("deadbeef00000000").as_ptr())) };
		let v: serde_json::Value = serde_json::from_str(&out).unwrap();
		assert!(v["error"].as_str().unwrap().contains("hash not found"));
		aphrodite_destroy(handle.as_ptr());
	}

	#[test]
	fn invalid_handles_are_rejected_without_crashing() {
		for bad in ["999999", "abc", ""] {
			let handle = cs(bad);
			for out in [
				unsafe { take(aphrodite_stats(handle.as_ptr())) },
				unsafe { take(aphrodite_catalog(handle.as_ptr(), cs("full").as_ptr())) },
				unsafe { take(aphrodite_search(handle.as_ptr(), cs("q").as_ptr())) },
				unsafe { take(aphrodite_config_get(handle.as_ptr(), cs("model").as_ptr())) },
				unsafe { take(aphrodite_session_start(handle.as_ptr())) },
				unsafe { take(aphrodite_compress(handle.as_ptr(), cs("x").as_ptr(), cs("text").as_ptr())) },
				unsafe { take(aphrodite_retrieve(handle.as_ptr(), cs("x").as_ptr())) },
			] {
				assert!(
					out.contains("invalid handle"),
					"expected invalid-handle error for {:?}, got {:?}",
					bad,
					out
				);
			}
			// Must not panic even though the handle is bogus.
			aphrodite_destroy(handle.as_ptr());
		}
	}

	#[test]
	fn classify_null_content_returns_error() {
		let out = unsafe { take(aphrodite_classify(std::ptr::null())) };
		let v: serde_json::Value = serde_json::from_str(&out).unwrap();
		assert_eq!(v["error"].as_str().unwrap(), "null content");
	}

	#[test]
	fn dispatch_known_and_unknown_hooks() {
		let h = unsafe { take(aphrodite_init(std::ptr::null())) };
		let handle = cs(&h);

		let ok = unsafe {
			take(aphrodite_dispatch(
				handle.as_ptr(),
				cs("session_start").as_ptr(),
				cs("{}").as_ptr(),
			))
		};
		assert!(serde_json::from_str::<serde_json::Value>(&ok).is_ok());

		let unknown = unsafe {
			take(aphrodite_dispatch(
				handle.as_ptr(),
				cs("not_a_real_hook").as_ptr(),
				cs("{}").as_ptr(),
			))
		};
		let v: serde_json::Value = serde_json::from_str(&unknown).unwrap();
		assert!(v["error"].as_str().unwrap().contains("unknown hook"));

		aphrodite_destroy(handle.as_ptr());
	}

	#[test]
	fn free_string_null_is_a_no_op() {
		aphrodite_free_string(std::ptr::null_mut());
	}

	#[test]
	fn oversize_content_is_rejected() {
		let h = unsafe { take(aphrodite_init(std::ptr::null())) };
		let handle = cs(&h);
		let big = "a".repeat(MAX_CONTENT + 1);
		let out = unsafe { take(aphrodite_compress(handle.as_ptr(), cs(&big).as_ptr(), cs("text").as_ptr())) };
		let v: serde_json::Value = serde_json::from_str(&out).unwrap();
		assert!(v["error"].as_str().unwrap().contains("16MB"));
		aphrodite_destroy(handle.as_ptr());
	}

	// ── T3 (F5): the seven previously-unguarded extern fns must survive a
	// panic without aborting - a deliberately panicking classify call must
	// come back as error JSON, not crash the test process. ──
	#[test]
	fn guarded_helper_converts_panic_to_error_json() {
		let ptr = guarded(|| panic!("deliberate test panic"));
		let out = unsafe { take(ptr) };
		let v: serde_json::Value = serde_json::from_str(&out).unwrap();
		assert!(v["error"].as_str().unwrap().contains("panicked"));
	}

	// ── T4 (F3): interior NUL bytes in stored/derived content must not
	// panic `CString::new(...).unwrap()` at any of the five raw-return
	// sites (retrieve, config_get, filter_lines, preview, stage2). A raw
	// C-string *input* can never carry a NUL (by definition), so the only
	// real ingress is via JSON args to `aphrodite_dispatch`, exactly as F3
	// describes: `{"content":"a\u0000b"}` decodes to a Rust `String`
	// containing a NUL byte.
	#[test]
	fn retrieve_tolerates_interior_nul_in_dispatched_content() {
		let h = unsafe { take(aphrodite_init(std::ptr::null())) };
		let handle = cs(&h);
		// Force compression regardless of size so a hash is always produced.
		let _ = unsafe {
			take(aphrodite_config_set(
				handle.as_ptr(),
				cs("tool_threshold").as_ptr(),
				cs("0").as_ptr(),
			))
		};
		let args = cs(r#"{"content":"x\u0000y","tool_name":"t"}"#);
		let dispatched = unsafe {
			take(aphrodite_dispatch(
				handle.as_ptr(),
				cs("transform_tool_result").as_ptr(),
				args.as_ptr(),
			))
		};
		let v: serde_json::Value = serde_json::from_str(&dispatched).unwrap();
		let hash = v["hash"].as_str().expect("compression should have produced a hash").to_string();
		// Must not panic even though the stored content contains a NUL byte.
		let retrieved = unsafe { take(aphrodite_retrieve(handle.as_ptr(), cs(&hash).as_ptr())) };
		assert_eq!(retrieved, "xy"); // NUL stripped, not left dangling
		aphrodite_destroy(handle.as_ptr());
	}
}
