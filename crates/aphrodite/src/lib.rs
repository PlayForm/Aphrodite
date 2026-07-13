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
pub mod hooks;
pub mod marker;
pub mod prefetch;
pub mod resolve;
pub mod session;
pub mod stage2;
pub mod state;
pub mod struct_extract;

// Proxy modules (used by main.rs binary)
pub mod center;
pub mod config;
pub mod proxy;
pub mod retrieve;
pub mod scripting;
pub mod setup;

use std::{
	collections::HashMap,
	ffi::{CStr, CString},
	os::raw::c_char,
	sync::Mutex,
};

use headroom_core::transforms;
use state::AphroditeState;

// ── Hardened primitives ──────────────────────────────────────────────

const MAX_CONTENT:usize = 16 * 1024 * 1024; // 16MB cap

/// Process-global handle table. `None` until the first handle is allocated,
/// so `Mutex::new` can stay `const` without requiring a heap alloc at
/// startup; `handles()` lazily initializes it to `Some` on first use.
static HANDLES:Mutex<Option<HashMap<usize, AphroditeState>>> = Mutex::new(None);
/// Next handle ID to hand out from `alloc_handle`. Wraps on overflow rather
/// than panicking - see the `wrapping_add` call there.
static NEXT_ID:Mutex<usize> = Mutex::new(1);

fn handles() -> std::sync::MutexGuard<'static, Option<HashMap<usize, AphroditeState>>> {
	let mut g = HANDLES.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	if g.is_none() {
		*g = Some(HashMap::new());
	}
	g
}

fn alloc_handle(state:AphroditeState) -> usize {
	let mut id = NEXT_ID.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
	let hid = *id;
	*id = id.wrapping_add(1); // overflow-safe
	handles().as_mut().unwrap().insert(hid, state);
	hid
}

fn with_state<T>(hid:usize, f:impl FnOnce(&mut AphroditeState) -> T) -> Result<T, String> {
	let mut h = handles();
	let state = match h.as_mut().and_then(|m| m.get_mut(&hid)) {
		Some(s) => s,
		None => return Err(format!("invalid handle: {}", hid)),
	};
	std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(state)))
		.map_err(|_| "internal error: hook panicked".to_string())
}

fn to_json_error(msg:&str) -> *mut c_char {
	let json = serde_json::json!({"error": msg}).to_string();
	CString::new(json).map(|c| c.into_raw()).unwrap_or(std::ptr::null_mut())
}

/// Run `f` under `catch_unwind`, converting a panic into an error-JSON
/// `*mut c_char` instead of unwinding across the `extern "C"` boundary
/// (which triggers the Rust runtime's forced process abort). `with_state`
/// already gives stateful fns this guarantee; this covers the seven
/// extern fns that don't go through `with_state` (classify, call_hook,
/// retrieve, filter_lines, preview, stage2, struct_extract) - previously
/// the only guard in this file was `with_state`'s, so these seven could
/// abort the host process on a panic (e.g. the byte-slicing panics in
/// struct_extract.rs/marker.rs, now fixed separately, but any future panic
/// in these paths would have the same effect).
fn guarded(f:impl FnOnce() -> *mut c_char + std::panic::UnwindSafe) -> *mut c_char {
	std::panic::catch_unwind(f).unwrap_or_else(|_| to_json_error("internal error: panicked"))
}

fn to_json_ok(v:&serde_json::Value) -> *mut c_char {
	CString::new(v.to_string())
		.map(|c| c.into_raw())
		.unwrap_or(std::ptr::null_mut())
}

unsafe fn cstr(ptr:*const c_char) -> Option<String> {
	if ptr.is_null() {
		return None;
	}
	Some(CStr::from_ptr(ptr).to_string_lossy().into_owned())
}

fn check_content(content:&str) -> Result<(), &'static str> {
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
pub extern "C" fn aphrodite_version() -> *mut c_char { CString::new(env!("CARGO_PKG_VERSION")).unwrap().into_raw() }

#[no_mangle]
pub extern "C" fn aphrodite_free_string(s:*mut c_char) {
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
pub extern "C" fn aphrodite_init(config_path:*const c_char) -> *mut c_char {
	let path = unsafe { cstr(config_path) }.unwrap_or_default();
	let mut state = AphroditeState::default();
	if !path.is_empty() {
		if let Ok(s) = std::fs::read_to_string(path.as_str()) {
			if let Ok(t) = s.parse::<toml::Table>() {
				if let Some(c) = t.get("compression").and_then(|v| v.as_table()) {
					if let Some(v) = c.get("context_engine").and_then(|v| v.as_bool()) {
						state.context_engine_enabled = v;
					}
					if let Some(v) = c.get("engine_threshold_pct").and_then(|v| v.as_integer()) {
						state.engine_threshold_pct = v as u64;
					}
					if let Some(v) = c.get("tool_threshold").and_then(|v| v.as_integer()) {
						state.tool_threshold = v as usize;
					}
					if let Some(v) = c.get("terminal_threshold").and_then(|v| v.as_integer()) {
						state.terminal_threshold = v as usize;
					}
				}
			}
		}
	}
	CString::new(alloc_handle(state).to_string()).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn aphrodite_destroy(handle:*const c_char) {
	if let Ok(hid) = unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		handles().as_mut().map(|m| m.remove(&hid));
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_classify(content:*const c_char) -> *mut c_char {
	let c = match unsafe { cstr(content) } {
		Some(s) => s,
		None => return to_json_error("null content"),
	};
	guarded(std::panic::AssertUnwindSafe(move || {
		if let Err(e) = check_content(&c) {
			return to_json_error(e);
		}
		let ct = transforms::detect(&c);
		to_json_ok(&serde_json::json!({"type":ct.as_str(),"lines":c.lines().count(),"bytes":c.len()}))
	}))
}

#[no_mangle]
pub extern "C" fn aphrodite_call_hook(hook:*const c_char, args:*const c_char) -> *mut c_char {
	let name = unsafe { cstr(hook) }.unwrap_or_default();
	let args_str = unsafe { cstr(args) }.unwrap_or_default();
	guarded(std::panic::AssertUnwindSafe(move || {
		let a:serde_json::Value = match serde_json::from_str(&args_str) {
			Ok(v) => v,
			Err(e) => return to_json_error(&format!("invalid args: {}", e)),
		};
		let content = a.get("content").and_then(|v| v.as_str()).unwrap_or("");
		let tool = a.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
		let mut s = AphroditeState::default();
		let r = match name.as_ref() {
			// Accept both the canonical Hermes name and the legacy alias.
			"on_session_start" | "session_start" => hooks::on_session_start(&mut s),
			"transform_tool_result" => hooks::transform_tool_result(&mut s, content, tool),
			"transform_terminal_output" => hooks::transform_terminal_output(&mut s, content),
			_ => serde_json::json!({"error": format!("unknown hook: {}", name)}),
		};
		to_json_ok(&r)
	}))
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

stateful!(aphrodite_compress, |s, content:*const c_char, hint:*const c_char| {
	if content.is_empty() {
		return serde_json::json!({"error":"empty"});
	}
	if content.len() > MAX_CONTENT {
		return serde_json::json!({"error":"content exceeds 16MB limit"});
	}
	let ct = transforms::detect(&content);
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
		hash:hash.clone(),
		ccr_type:t.clone(),
		size:content.len(),
		preview:preview.clone(),
		turn:s.turn_counter,
		center:None,
		meta:None,
	});
	serde_json::json!({"hash":hash,"type":t,"size":content.len(),"preview":preview,"marker":marker})
});

// aphrodite_retrieve is a manual override below - returns raw content, not JSON

// Override: retrieve returns raw content, not JSON-wrapped
#[no_mangle]
pub extern "C" fn aphrodite_retrieve(handle:*const c_char, hash:*const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let hash = unsafe { cstr(hash) }.unwrap_or_default();
	guarded(std::panic::AssertUnwindSafe(move || {
		let mut h = handles();
		match h.as_mut().and_then(|m| m.get_mut(&hid)) {
			Some(s) => {
				match s.inline_store_get(&hash) {
					Some(content) => CString::new(content.replace('\0', "")).unwrap().into_raw(),
					None => to_json_error(&format!("hash not found: {}", hash)),
				}
			},
			None => to_json_error(&format!("invalid handle: {}", hid)),
		}
	}))
}

stateful!(aphrodite_transform, |s, content:*const c_char, tool:*const c_char| {
	if content.len() > MAX_CONTENT {
		return serde_json::json!({"error":"content exceeds 16MB limit"});
	}
	hooks::transform_tool_result(s, &content, &tool)
});

stateful!(aphrodite_terminal, |s, content:*const c_char| {
	if content.len() > MAX_CONTENT {
		return serde_json::json!({"error":"content exceeds 16MB limit"});
	}
	hooks::transform_terminal_output(s, &content)
});

#[no_mangle]
pub extern "C" fn aphrodite_session_start(handle:*const c_char) -> *mut c_char {
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
pub extern "C" fn aphrodite_catalog(handle:*const c_char, mode:*const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let m = unsafe { cstr(mode) }.unwrap_or_default();
	let h = handles();
	match h.as_ref().and_then(|map| map.get(&hid)) {
		Some(s) => {
			// NOTE: both "toc" and "full" modes emit the full 40-char hash -
			// a truncated hash is unresolvable via exact-match retrieval
			// (report 05 F3). Truncate only for human-readable display
			// (e.g. `catalog::format_catalog_table`), never in machine-
			// consumed JSON like this.
			let items:Vec<serde_json::Value> = s
				.recent_markers
				.iter()
				.map(|e| {
					if m == "toc" {
						serde_json::json!({"hash":&e.hash,"type":e.ccr_type,"size":e.size,"preview":e.preview})
					} else {
						serde_json::json!({"hash":e.hash,"type":e.ccr_type,"size":e.size,"preview":e.preview,"turn":e.turn})
					}
				})
				.collect();
			to_json_ok(&serde_json::json!({"total":items.len(),"items":items,"turn":s.turn_counter}))
		},
		None => to_json_error(&format!("invalid handle: {}", hid)),
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_stats(handle:*const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let h = handles();
	match h.as_ref().and_then(|map| map.get(&hid)) {
		Some(s) => {
			to_json_ok(&serde_json::json!({
				"version":env!("CARGO_PKG_VERSION"),"inline_entries":s.inline_store.len(),
				"markers":s.recent_markers.len(),"turn":s.turn_counter,
				"engine_enabled":s.context_engine_enabled,"threshold_pct":s.engine_threshold_pct,
				"tool_threshold":s.tool_threshold,"terminal_threshold":s.terminal_threshold,
			}))
		},
		None => to_json_error(&format!("invalid handle: {}", hid)),
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_reload(handle:*const c_char, path:*const c_char) -> *mut c_char {
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
pub extern "C" fn aphrodite_search(handle:*const c_char, query:*const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let q = unsafe { cstr(query) }.unwrap_or_default().to_lowercase();
	let h = handles();
	match h.as_ref().and_then(|map| map.get(&hid)) {
		Some(s) => {
			let results:Vec<serde_json::Value> = s
				.recent_markers
				.iter()
				.filter(|m| m.preview.to_lowercase().contains(&q) || m.ccr_type.to_lowercase().contains(&q))
				.take(20)
				.map(|m| serde_json::json!({"hash":&m.hash,"type":m.ccr_type,"size":m.size,"preview":m.preview}))
				.collect();
			to_json_ok(&serde_json::json!({"total":results.len(),"results":results}))
		},
		None => to_json_error(&format!("invalid handle: {}", hid)),
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_config_get(handle:*const c_char, key:*const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let k = unsafe { cstr(key) }.unwrap_or_default();
	let h = handles();
	match h.as_ref().and_then(|map| map.get(&hid)) {
		Some(s) => {
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
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_config_set(handle:*const c_char, key:*const c_char, value:*const c_char) -> *mut c_char {
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

/// Detect the CCR content-type string for a blob (e.g. `source_code`, `build`,
/// `json_array`). Thin wrapper over the Headroom classifier so downstream
/// crates (aphrodite-hermes) don't need a direct headroom-core dependency.
pub fn detect_type(content:&str) -> String { transforms::detect(content).as_str().to_string() }

/// Build a compact, human-readable preview string for compressed content,
/// shaped per content type (e.g. error/warning counts for build output,
/// +/- line counts for diffs, fn/struct counts for source code) so the LLM
/// gets a useful summary instead of a generic byte/line count wherever a
/// richer signal is available.
pub fn build_preview(type_str:&str, content:&str) -> String {
	let lines = content.lines().count();
	let bytes = content.len();
	match type_str {
		"build" | "build_output" | "build_error" => {
			let e = content.matches("error").count();
			let w = content.matches("warning").count();
			format!("[{}:{}E {}W {}L]", type_str, e, w, lines)
		},
		"diff" => {
			let f = content.matches("diff --git").count();
			let a = content.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
			let d = content.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
			format!("[diff:{}F +{}/-{} {}L]", f, a, d, lines)
		},
		"source_code" | "code_rust" | "code_python" | "code_go" | "code_js" | "code_ts" => {
			// Enrich with the structure map (fns/structs/traits/impls/classes/types
			// + first signature) so the dylib/hook path matches the proxy's preview
			// quality, instead of a bare substring count.
			let st = crate::struct_extract::extract_code_structure(content, "");
			let mut parts:Vec<String> = Vec::new();
			for (key, label) in [
				("fns", "fns"),
				("structs", "structs"),
				("traits", "traits"),
				("impls", "impls"),
				("classes", "classes"),
				("types", "types"),
			] {
				if let Some(v) = st.get(key) {
					if !v.is_empty() {
						parts.push(format!("{}{}", v.len(), label));
					}
				}
			}
			let summary = if parts.is_empty() {
				format!("{}fns", content.matches("fn ").count() + content.matches("def ").count())
			} else {
				parts.join("|")
			};
			let sig = st
				.get("fns")
				.and_then(|v| v.first())
				.map(|s| format!(" {}", s.chars().take(48).collect::<String>().trim()))
				.unwrap_or_default();
			format!("[code:{}{} {}L]", summary, sig, lines)
		},
		"search" => {
			let h = content.lines().filter(|l| l.contains(':')).count();
			format!("[search:{}hits {}L]", h, lines)
		},
		"json_array" => {
			let i = content.matches("{\"").count();
			format!("[json:{}items {}L]", i, lines)
		},
		// `hooks::transform_terminal_output` overrides the classified type to
		// "terminal" when the content looks like a shell/exit-code trace, but
		// this function had no matching arm for it, so the preview silently
		// fell through to the generic `_` branch (F10) - a bare line/byte
		// count with no exit-code or last-output-line context, the exact
		// signal a terminal preview exists to surface.
		"terminal" => {
			let exit_line = content
				.lines()
				.rev()
				.find(|l| l.contains("exit code:") || l.contains("Error:"))
				.map(|l| l.trim());
			let last_line = content.lines().rev().find(|l| !l.trim().is_empty()).map(|l| l.trim());
			let summary = exit_line.or(last_line).unwrap_or("").chars().take(60).collect::<String>();
			format!("[terminal:{}L {}]", lines, summary)
		},
		_ => format!("[{}:{}L {}B]", type_str, lines, bytes),
	}
}

// ── Universal dispatch: all Python hooks route through here ──

/// Universal hook dispatcher. Python calls this for every hook handler.
/// Returns JSON-wrapped result or raw string if content-only.
#[no_mangle]
pub extern "C" fn aphrodite_dispatch(
	handle:*const c_char,
	hook_name:*const c_char,
	args_json:*const c_char,
) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let name = unsafe { cstr(hook_name) }.unwrap_or_default();
	let args_str = unsafe { cstr(args_json) }.unwrap_or_default();

	let args:serde_json::Value = match serde_json::from_str(&args_str) {
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
		let path_strings:Vec<String> = paths.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
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
				let items:Vec<serde_json::Value> = s
					.recent_markers
					.iter()
					.map(|e| {
						if mode == "toc" {
							serde_json::json!({"hash":&e.hash,"type":e.ccr_type,"size":e.size,"preview":e.preview})
						} else {
							serde_json::json!({"hash":e.hash,"type":e.ccr_type,"size":e.size,"preview":e.preview,"turn":e.turn})
						}
					})
					.collect();
				serde_json::json!({"total":items.len(),"items":items,"turn":s.turn_counter})
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
			"search" => {
				let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
				let type_filter = args.get("type").and_then(|v| v.as_str());
				let results:Vec<serde_json::Value> = s
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
				let turns:Vec<serde_json::Value> = s
					.conv_index
					.iter()
					.map(
						|(turn, (hash, summary, size))| serde_json::json!({"turn":turn,"hash":hash,"summary":summary,"size":size}),
					)
					.collect();
				serde_json::json!({"turns":turns,"total":turns.len()})
			},
			"files" => {
				let files:Vec<serde_json::Value> = s
					.referenced_files
					.iter()
					.map(|(path, tool)| serde_json::json!({"path":path,"tool":tool}))
					.collect();
				serde_json::json!({"files":files,"total":files.len()})
			},
			"classify" => {
				let ct = headroom_core::transforms::detect(content);
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
pub extern "C" fn aphrodite_filter_lines(content:*const c_char, query:*const c_char) -> *mut c_char {
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
pub extern "C" fn aphrodite_resolve(handle:*const c_char, hash:*const c_char) -> *mut c_char {
	let hid = match unsafe { cstr(handle) }.unwrap_or_default().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let h = unsafe { cstr(hash) }.unwrap_or_default();
	match with_state(hid, |s| {
		match crate::resolve::expand(s, &h) {
			Some(content) => serde_json::json!({"found":true,"content":content}),
			None => serde_json::json!({"found":false}),
		}
	}) {
		Ok(v) => to_json_ok(&v),
		Err(e) => to_json_error(&e),
	}
}

/// Generate preview for content - port of _marker/preview.py
#[no_mangle]
pub extern "C" fn aphrodite_preview(content:*const c_char, ccr_type:*const c_char) -> *mut c_char {
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
pub extern "C" fn aphrodite_stage2(content:*const c_char, ccr_type:*const c_char) -> *mut c_char {
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
pub extern "C" fn aphrodite_struct_extract(content:*const c_char, language:*const c_char) -> *mut c_char {
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

	fn cs(s:&str) -> CString { CString::new(s).unwrap() }

	unsafe fn take(ptr:*mut c_char) -> String {
		assert!(!ptr.is_null(), "expected non-null C string");
		let s = CStr::from_ptr(ptr).to_string_lossy().into_owned();
		aphrodite_free_string(ptr);
		s
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
		let v:serde_json::Value = serde_json::from_str(&compress_json).unwrap();
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
		let v:serde_json::Value = serde_json::from_str(&out).unwrap();
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
		let v:serde_json::Value = serde_json::from_str(&out).unwrap();
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
		let v:serde_json::Value = serde_json::from_str(&unknown).unwrap();
		assert!(v["error"].as_str().unwrap().contains("unknown hook"));

		aphrodite_destroy(handle.as_ptr());
	}

	#[test]
	fn free_string_null_is_a_no_op() { aphrodite_free_string(std::ptr::null_mut()); }

	#[test]
	fn oversize_content_is_rejected() {
		let h = unsafe { take(aphrodite_init(std::ptr::null())) };
		let handle = cs(&h);
		let big = "a".repeat(MAX_CONTENT + 1);
		let out = unsafe { take(aphrodite_compress(handle.as_ptr(), cs(&big).as_ptr(), cs("text").as_ptr())) };
		let v:serde_json::Value = serde_json::from_str(&out).unwrap();
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
		let v:serde_json::Value = serde_json::from_str(&out).unwrap();
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
		let v:serde_json::Value = serde_json::from_str(&dispatched).unwrap();
		let hash = v["hash"].as_str().expect("compression should have produced a hash").to_string();
		// Must not panic even though the stored content contains a NUL byte.
		let retrieved = unsafe { take(aphrodite_retrieve(handle.as_ptr(), cs(&hash).as_ptr())) };
		assert_eq!(retrieved, "xy"); // NUL stripped, not left dangling
		aphrodite_destroy(handle.as_ptr());
	}
}
