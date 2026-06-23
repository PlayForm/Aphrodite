//! headroom-ffi: Full Aphrodite plugin runtime as a C ABI cdylib.
//! 14 functions - init, destroy, classify, compress, retrieve, transform,
//! terminal, session_start, catalog, stats, reload, config_get/set, search.

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

static HANDLES:Mutex<Option<HashMap<usize, AphroditeState>>> = Mutex::new(None);
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

fn with_state<T>(hid:usize, f:impl FnOnce(&mut AphroditeState) -> T + std::panic::UnwindSafe) -> Result<T, String> {
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
	CString::new(
		serde_json::json!([
			"session_start",
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
	let path = unsafe { CStr::from_ptr(config_path) }.to_string_lossy();
	let mut state = AphroditeState::default();
	if !path.is_empty() {
		if let Ok(s) = std::fs::read_to_string(path.as_ref()) {
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
	if let Ok(hid) = unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() {
		handles().as_mut().map(|m| m.remove(&hid));
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_classify(content:*const c_char) -> *mut c_char {
	let c = match unsafe { cstr(content) } {
		Some(s) => s,
		None => return to_json_error("null content"),
	};
	if let Err(e) = check_content(&c) {
		return to_json_error(e);
	}
	let ct = transforms::detect(&c);
	to_json_ok(&serde_json::json!({"type":ct.as_str(),"lines":c.lines().count(),"bytes":c.len()}))
}

#[no_mangle]
pub extern "C" fn aphrodite_call_hook(hook:*const c_char, args:*const c_char) -> *mut c_char {
	let name = unsafe { CStr::from_ptr(hook) }.to_string_lossy();
	let args_str = unsafe { CStr::from_ptr(args) }.to_string_lossy();
	let a:serde_json::Value = match serde_json::from_str(&args_str) {
		Ok(v) => v,
		Err(e) => return to_json_error(&format!("invalid args: {}", e)),
	};
	let content = a.get("content").and_then(|v| v.as_str()).unwrap_or("");
	let tool = a.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
	let mut s = AphroditeState::default();
	let r = match name.as_ref() {
		"session_start" => hooks::on_session_start(&mut s),
		"transform_tool_result" => hooks::transform_tool_result(&mut s, content, tool),
		"transform_terminal_output" => hooks::transform_terminal_output(&mut s, content),
		_ => serde_json::json!({"error": format!("unknown hook: {}", name)}),
	};
	to_json_ok(&r)
}

// ── Stateful operations ──────────────────────────────────────────────

macro_rules! stateful {
    ($name:ident, |$s:ident, $($arg:ident : $ty:ty),*| $body:expr) => {
        #[no_mangle] pub extern "C" fn $name(handle: *const c_char, $($arg: *const c_char),*) -> *mut c_char {
            let hid = match unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() { Ok(id) => id, Err(_) => return to_json_error("invalid handle") };
            $(let $arg = unsafe { CStr::from_ptr($arg) }.to_string_lossy();)*
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
	let hid = match unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let hash = unsafe { CStr::from_ptr(hash) }.to_string_lossy();
	let mut h = handles();
	match h.as_mut().and_then(|m| m.get_mut(&hid)) {
		Some(s) => {
			match s.inline_store_get(&hash) {
				Some(content) => CString::new(content).unwrap().into_raw(),
				None => to_json_error(&format!("hash not found: {}", hash)),
			}
		},
		None => to_json_error(&format!("invalid handle: {}", hid)),
	}
}

stateful!(aphrodite_transform, |s, content:*const c_char, tool:*const c_char| {
	hooks::transform_tool_result(s, &content, &tool)
});

stateful!(aphrodite_terminal, |s, content:*const c_char| {
	hooks::transform_terminal_output(s, &content)
});

#[no_mangle]
pub extern "C" fn aphrodite_session_start(handle:*const c_char) -> *mut c_char {
	let hid = match unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	match with_state(hid, |s| session::on_session_start(s)) {
		Ok(v) => to_json_ok(&v),
		Err(e) => to_json_error(&e),
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_catalog(handle:*const c_char, mode:*const c_char) -> *mut c_char {
	let hid = match unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let m = unsafe { CStr::from_ptr(mode) }.to_string_lossy();
	let h = handles();
	match h.as_ref().and_then(|map| map.get(&hid)) {
		Some(s) => {
			let items:Vec<serde_json::Value> = s
				.recent_markers
				.iter()
				.map(|e| {
					if m == "toc" {
						serde_json::json!({"hash":&e.hash[..12.min(e.hash.len())],"type":e.ccr_type,"size":e.size,"preview":e.preview})
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
	let hid = match unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() {
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
	let hid = match unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let p = unsafe { CStr::from_ptr(path) }.to_string_lossy();
	match with_state(hid, |s| {
		if !p.is_empty() {
			if let Ok(t) = std::fs::read_to_string(p.as_ref()) {
				if let Ok(tbl) = t.parse::<toml::Table>() {
					if let Some(c) = tbl.get("compression").and_then(|v| v.as_table()) {
						if let Some(v) = c.get("context_engine").and_then(|v| v.as_bool()) {
							s.context_engine_enabled = v;
						}
					}
				}
			}
		}
		serde_json::json!({"status":"ok"})
	}) {
		Ok(v) => to_json_ok(&v),
		Err(e) => to_json_error(&e),
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_search(handle:*const c_char, query:*const c_char) -> *mut c_char {
	let hid = match unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let q = unsafe { CStr::from_ptr(query) }.to_string_lossy().to_lowercase();
	let h = handles();
	match h.as_ref().and_then(|map| map.get(&hid)) {
		Some(s) => {
			let results: Vec<serde_json::Value> = s.recent_markers.iter()
                .filter(|m| m.preview.to_lowercase().contains(&q) || m.ccr_type.to_lowercase().contains(&q))
                .take(20).map(|m| serde_json::json!({"hash":&m.hash[..12.min(m.hash.len())],"type":m.ccr_type,"size":m.size,"preview":m.preview})).collect();
			to_json_ok(&serde_json::json!({"total":results.len(),"results":results}))
		},
		None => to_json_error(&format!("invalid handle: {}", hid)),
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_config_get(handle:*const c_char, key:*const c_char) -> *mut c_char {
	let hid = match unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let k = unsafe { CStr::from_ptr(key) }.to_string_lossy();
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
			CString::new(v).unwrap().into_raw()
		},
		None => to_json_error(&format!("invalid handle: {}", hid)),
	}
}

#[no_mangle]
pub extern "C" fn aphrodite_config_set(handle:*const c_char, key:*const c_char, value:*const c_char) -> *mut c_char {
	let hid = match unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() {
		Ok(id) => id,
		Err(_) => return to_json_error("invalid handle"),
	};
	let k = unsafe { CStr::from_ptr(key) }.to_string_lossy();
	let v = unsafe { CStr::from_ptr(value) }.to_string_lossy();
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
		"source_code" => {
			let f = content.matches("fn ").count() + content.matches("def ").count();
			format!("[code:{}fns {}L]", f, lines)
		},
		"search" => {
			let h = content.lines().filter(|l| l.contains(':')).count();
			format!("[search:{}hits {}L]", h, lines)
		},
		"json_array" => {
			let i = content.matches("{\"").count();
			format!("[json:{}items {}L]", i, lines)
		},
		_ => format!("[{}:{}L {}B]", type_str, lines, bytes),
		}
		}

		// ── Universal dispatch: all Python hooks route through here ──

		/// Universal hook dispatcher. Python calls this for every hook handler.
		/// Returns JSON-wrapped result or raw string if content-only.
		#[no_mangle]
		pub extern "C" fn aphrodite_dispatch(
		handle: *const c_char,
		hook_name: *const c_char,
		args_json: *const c_char,
		) -> *mut c_char {
		let hid = match unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() {
		    Ok(id) => id,
		    Err(_) => return to_json_error("invalid handle"),
		};
		let name = unsafe { CStr::from_ptr(hook_name) }.to_string_lossy();
		let args_str = unsafe { CStr::from_ptr(args_json) }.to_string_lossy();

		let args: serde_json::Value =
		    match serde_json::from_str(&args_str) {
		        Ok(v) => v,
		        Err(e) => return to_json_error(&format!("invalid args: {}", e)),
		    };

		let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
		let tool = args.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");

		let result = match with_state(hid, |s| match name.as_ref() {
		    "session_start" => hooks::on_session_start(s),
		    "transform_tool_result" => hooks::transform_tool_result(s, content, tool),
		    "transform_terminal_output" => hooks::transform_terminal_output(s, content),
		    "pre_llm_call" => hooks::pre_llm_call(s),
		    "post_llm_call" => hooks::post_llm_call(s),
		    "catalog" => {
		        let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("full");
		        let items: Vec<serde_json::Value> = s.recent_markers.iter().map(|e| {
		            if mode == "toc" {
		                serde_json::json!({"hash":&e.hash[..12.min(e.hash.len())],"type":e.ccr_type,"size":e.size,"preview":e.preview})
		            } else {
		                serde_json::json!({"hash":e.hash,"type":e.ccr_type,"size":e.size,"preview":e.preview,"turn":e.turn})
		            }
		        }).collect();
		        serde_json::json!({"total":items.len(),"items":items,"turn":s.turn_counter})
		    }
		    "stats" => serde_json::json!({
		        "version": env!("CARGO_PKG_VERSION"),
		        "inline_entries": s.inline_store.len(),
		        "markers": s.recent_markers.len(),
		        "turn": s.turn_counter,
		        "engine_enabled": s.context_engine_enabled,
		    }),
		    "search" => {
		        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
		        let type_filter = args.get("type").and_then(|v| v.as_str());
		        let results: Vec<serde_json::Value> = s.recent_markers.iter()
		            .filter(|m| {
		                let matches_query = query.is_empty()
		                    || m.preview.to_lowercase().contains(&query)
		                    || m.ccr_type.to_lowercase().contains(&query);
		                let matches_type = type_filter.map_or(true, |t| m.ccr_type == t);
		                matches_query && matches_type
		            })
		            .take(20)
		            .map(|m| serde_json::json!({"hash":&m.hash[..12.min(m.hash.len())],"type":m.ccr_type,"size":m.size,"preview":m.preview}))
		            .collect();
		        serde_json::json!({"total":results.len(),"results":results})
		    }
		    "diff" => {
		        let turns: Vec<serde_json::Value> = s.conv_index.iter().map(|(turn, (hash, summary, size))| {
		            serde_json::json!({"turn":turn,"hash":hash,"summary":summary,"size":size})
		        }).collect();
		        serde_json::json!({"turns":turns,"total":turns.len()})
		    }
		    "files" => {
		        let files: Vec<serde_json::Value> = s.referenced_files.iter().map(|(path, tool)| {
		            serde_json::json!({"path":path,"tool":tool})
		        }).collect();
		        serde_json::json!({"files":files,"total":files.len()})
		    }
		    "classify" => {
		        let ct = headroom_core::transforms::detect(content);
		        serde_json::json!({"type":ct.as_str(),"lines":content.lines().count(),"bytes":content.len()})
		    }
		    "prefetch" => {
		        let paths = args.get("paths").and_then(|v| v.as_array()).cloned().unwrap_or_default();
		        let path_strings: Vec<String> = paths.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
		        let r = crate::prefetch::prefetch_files(s, &path_strings);
		        r
		    }
		    _ => serde_json::json!({"error": format!("unknown hook: {}", name)}),
		}) {
		    Ok(v) => to_json_ok(&v),
		    Err(e) => to_json_error(&e),
		};

		result
		}

		/// Filter lines by query - port of _resolve.py _filter_lines
		#[no_mangle]
		pub extern "C" fn aphrodite_filter_lines(
		content: *const c_char,
		query: *const c_char,
		) -> *mut c_char {
		let c = match unsafe { cstr(content) } { Some(s) => s, None => return to_json_error("null content") };
		let q = unsafe { CStr::from_ptr(query) }.to_string_lossy();
		let filtered = crate::resolve::filter_lines(&c, &q);
		CString::new(filtered).unwrap().into_raw()
		}

		/// Resolve hash with full recursive expansion - port of _resolve.py
		#[no_mangle]
		pub extern "C" fn aphrodite_resolve(
		handle: *const c_char,
		hash: *const c_char,
		) -> *mut c_char {
		let hid = match unsafe { CStr::from_ptr(handle) }.to_string_lossy().parse::<usize>() {
		    Ok(id) => id, Err(_) => return to_json_error("invalid handle")
		};
		let h = unsafe { CStr::from_ptr(hash) }.to_string_lossy();
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
		pub extern "C" fn aphrodite_preview(
		content: *const c_char,
		ccr_type: *const c_char,
		) -> *mut c_char {
		let c = match unsafe { cstr(content) } { Some(s) => s, None => return to_json_error("null content") };
		let t = unsafe { CStr::from_ptr(ccr_type) }.to_string_lossy();
		let preview = crate::build_preview(&t, &c);
		CString::new(preview).unwrap().into_raw()
		}

		/// Stage 2 semantic reduction - port of _stage2.py
		#[no_mangle]
		pub extern "C" fn aphrodite_stage2(
		content: *const c_char,
		ccr_type: *const c_char,
		) -> *mut c_char {
		let c = match unsafe { cstr(content) } { Some(s) => s, None => return to_json_error("null content") };
		let t = unsafe { CStr::from_ptr(ccr_type) }.to_string_lossy();
		match crate::stage2::compress_stage2(&c, &t) {
		    Some(reduced) => CString::new(reduced).unwrap().into_raw(),
		    None => to_json_error("no reduction possible"),
		}
		}

		/// Code structure extraction - port of _core/struct.py
		#[no_mangle]
		pub extern "C" fn aphrodite_struct_extract(
		content: *const c_char,
		language: *const c_char,
		) -> *mut c_char {
		let c = match unsafe { cstr(content) } { Some(s) => s, None => return to_json_error("null content") };
		let lang = unsafe { CStr::from_ptr(language) }.to_string_lossy();
		let result = crate::struct_extract::extract_code_structure(&c, &lang);
		to_json_ok(&serde_json::json!(result))
		}
