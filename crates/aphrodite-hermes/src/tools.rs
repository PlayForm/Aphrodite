//! Tool dispatch - routes Hermes tool calls to aphrodite core functions.
//!
//! Each Hermes tool (`aphrodite_compress`, `aphrodite_retrieve`, ...) has a
//! handler here that parses args, operates on the process-global session state
//! (see [`crate::with_shared`]), and returns a JSON result. Because every
//! handler shares one state, content compressed by a hook or
//! `aphrodite_compress` stays resolvable by `aphrodite_retrieve` for the life
//! of the session.

use std::collections::HashMap;

use aphrodite::state::{AphroditeState, MarkerEntry};

use crate::{proxy_health, with_shared};

type ToolHandler = fn(args:&serde_json::Value) -> serde_json::Value;

/// Dispatch a tool by name. Returns `{"error": "..."}` for unknown tools.
pub fn dispatch(name:&str, args_json:&str) -> serde_json::Value {
	let registry = tool_registry();
	match registry.get(name) {
		Some(handler) => {
			let args:serde_json::Value = match serde_json::from_str(args_json) {
				Ok(v) => v,
				Err(e) => return serde_json::json!({"error": format!("invalid args: {}", e)}),
			};
			handler(&args)
		},
		None => serde_json::json!({"error": format!("unknown tool: {}", name)}),
	}
}

// ── Shared helpers ─────────────────────────────────────────

fn str_arg<'a>(args:&'a serde_json::Value, key:&str) -> &'a str { args.get(key).and_then(|v| v.as_str()).unwrap_or("") }

/// Largest file `aphrodite_retrieve(path=…)` will read directly.
const MAX_PATH_READ:u64 = 10 * 1024 * 1024;

/// Read a file requested via `aphrodite_retrieve(path=…)`, confined to the
/// current workspace and capped at [`MAX_PATH_READ`]. Returns `Err(reason)` if
/// the path escapes the workspace, is too large, or can't be read - so the tool
/// can't be coerced into exfiltrating arbitrary files (e.g. /etc/passwd,
/// ~/.ssh).
fn read_path_guarded(path:&str) -> Result<String, String> {
	let root = std::env::current_dir().map_err(|e| format!("cwd: {e}"))?;
	let root = root.canonicalize().unwrap_or(root);
	let canon = std::path::Path::new(path)
		.canonicalize()
		.map_err(|e| format!("read {path}: {e}"))?;
	if !canon.starts_with(&root) {
		return Err(format!("path is outside the workspace ({}): {path}", root.display()));
	}
	let size = std::fs::metadata(&canon).map(|m| m.len()).unwrap_or(0);
	if size > MAX_PATH_READ {
		return Err(format!("file exceeds {MAX_PATH_READ}-byte read cap: {path}"));
	}
	std::fs::read_to_string(&canon).map_err(|e| format!("read {path}: {e}"))
}

/// Hermes wraps all tool results in JSON wrappers like
/// {"output":"...","exit_code":N}, {"success":true,"diff":"..."},
/// {"total_count":N,"matches":[...]}, etc. The aphrodite classifier sees a
/// JSON object and types it as text (json_array only for '[' array shapes) -
/// hiding the real content behind a useless preview. This extracts the
/// meaningful content and reclassifies.
pub(crate) fn unwrap_hermes_result(content:&str) -> Option<(String, String)> {
	// Only attempt unwrapping if the content looks like a JSON object.
	if !content.trim_start().starts_with('{') {
		return None;
	}
	let v:serde_json::Value = serde_json::from_str(content).ok()?;
	let obj = v.as_object()?;

	// ── Terminal: {"output":"...","exit_code":N,"error":null} ──
	// A non-empty `output` always wins (checked first, below); an empty one
	// falls through to the `error`/`success` branches instead of aborting
	// extraction entirely (F12) - a failed command with empty stdout is
	// exactly the case where the error string IS the payload.
	if let (Some(output), Some(_exit)) = (obj.get("output").and_then(|o| o.as_str()), obj.get("exit_code"))
		&& !output.is_empty()
	{
		// Terminal outputs are often short and don't trigger the headroom
		// classifier's build_output pattern. Add explicit heuristics so
		// cargo output, test runs, and shell traces get meaningful previews.
		let ct:String = if output.contains("exit code:") || output.contains("Error:") {
			"terminal".into()
		} else if output.contains("   Compiling")
			|| output.contains("    Finished")
			|| output.contains("   Running")
			|| output.contains("test result:")
			|| output.contains("   Building")
			|| output.contains("   Installing")
			|| output.contains("warning:")
			|| output.contains("error[")
		{
			if output.contains("error[") || output.contains("error: could not") {
				"build_error".into()
			} else {
				"build_output".into()
			}
		} else {
			aphrodite::detect_type(output)
		};
		return Some((output.to_string(), ct));
	}

	// ── Patch / write_file: {"success":...,"diff":"...","error":"..."} ──
	if let Some(diff) = obj.get("diff").and_then(|d| d.as_str())
		&& !diff.is_empty()
	{
		return Some((diff.to_string(), aphrodite::detect_type(diff)));
	}
	if let Some(msg) = obj.get("error").and_then(|m| m.as_str()) {
		if msg.starts_with("Found") && msg.contains("matches") {
			return Some((msg.to_string(), "text".to_string()));
		}
		if !msg.is_empty() && !msg.starts_with('{') && !msg.starts_with('[') {
			return Some((msg.to_string(), "text".to_string()));
		}
	}
	if let Some(ok) = obj.get("success") {
		// ISSUE-11 WS1: the bare `{"success": true}` collapse
		// (Some(("ok", "text"))) is deleted - it fired on ANY success-plus-
		// one-key envelope and replaced the whole payload with a 2-byte
		// fragment, producing the reported `[text:1L 2B | ok]`. Every
		// genuine Hermes envelope is caught by the branches above
		// (output/exit_code, diff, error), so a success-only object is a
		// user payload: let it fall through to detect_type / the caller
		// hint. The success-STRING branch survives only for SINGLE-key
		// objects (e.g. {"success": "wrote 3 files"}) - multi-key payloads
		// like {"success": "ok", "data": [...]} must not collapse to the
		// word either.
		if let Some(msg) = ok.as_str()
			&& obj.len() <= 1
			&& !msg.is_empty()
			&& !msg.starts_with('{')
		{
			return Some((msg.to_string(), "text".to_string()));
		}
	}

	// ── Search: {"total_count":N,"matches":[...] or "matches_text", "truncated":bool} ──
	if let Some(count) = obj.get("total_count").and_then(|c| c.as_u64()) {
		let truncated = obj.get("truncated").and_then(|t| t.as_bool()).unwrap_or(false);
		// ISSUE-11 WS2: build ONE grep-style line per REAL match and let
		// the search preview count them. The old take(20) cap made a
		// 5,000-hit search preview as "[search:20 hits ...]" - a count
		// that lied about the total. The lines feed only the preview
		// (never stored/hashed), so no size cap is needed.
		let mut lines:Vec<String> = Vec::new();
		if let Some(matches) = obj.get("matches").and_then(|m| m.as_array()) {
			for m in matches {
				if let (Some(p), Some(l)) = (m.get("path").or(m.get("file")).and_then(|v| v.as_str()), m.get("line")) {
					let content = m.get("content").and_then(|c| c.as_str()).unwrap_or("");
					lines.push(format!("{}:{}:{}", p, l, content));
				}
			}
		}
		// Real Hermes search_files results ship `matches_text` (path-
		// grouped: "path\n  <line>: <content>") instead of a `matches`
		// array. The old code ignored it, so every real search collapsed
		// to "[search:1L]" ("N total" has no file:line: rows for the
		// preview regex to count). Normalize it to the same grep-style
		// lines so the preview shows the real hit count and files.
		if lines.is_empty()
			&& let Some(text) = obj.get("matches_text").and_then(|t| t.as_str())
		{
			let mut cur_path:Option<&str> = None;
			for raw in text.lines() {
				let t = raw.trim();
				if t.is_empty() {
					continue;
				}
				if raw.trim_start().len() != raw.len() {
					// Indented row under the current path: "<line>: <content>".
					if let (Some(p), Some((lno, rest))) = (cur_path, t.split_once(':')) {
						lines.push(format!("{}:{}:{}", p, lno.trim(), rest.trim_start()));
					}
				} else {
					cur_path = Some(t);
				}
			}
		}
		if lines.is_empty() {
			// No per-match content to count. Surface the REAL total and
			// the truncated flag as the payload itself, typed `text` so
			// the generic preview arm renders "0 total" / "N total
			// (truncated)" instead of an unreadable "[search:1L]". Only
			// do this for genuine search shapes - a data object that
			// merely carries a total_count key (e.g. {"total_count": 42,
			// "items": [...]}) falls through and previews as itself.
			let is_search_shape = obj.get("matches").is_some()
				|| obj.get("matches_text").is_some()
				|| obj.get("matches_format").is_some()
				|| truncated;
			if is_search_shape {
				let label = if truncated {
					format!("{} total (truncated)", count)
				} else {
					format!("{} total", count)
				};
				return Some((label, "text".to_string()));
			}
		} else {
			return Some((lines.join("\n"), "search".to_string()));
		}
	}

	// ── File read: {"content":"...","total_lines":N} ──
	// (read_file is usually essential-skipped, but handle it anyway.)
	if let Some(text) = obj.get("content").and_then(|c| c.as_str()) {
		return Some((text.to_string(), aphrodite::detect_type(text)));
	}

	// ── Other Hermes tools: extract first meaningful string field ──
	// E.g. skill_view: {"name":"...","description":"..."}
	//       aphrodite_retrieve: {"found":true,"content":"..."}
	// ISSUE-11 WS1: only SINGLE-key objects collapse here. A multi-key
	// payload like {"result": "ok", "data": {...}} must preview as
	// itself, not as the fragment "ok" (the same bug class as the
	// deleted success-bool collapse, ungated by any size before).
	let priority_keys = ["description", "summary", "result", "message", "preview", "found"];
	if obj.len() <= 1 {
		for key in &priority_keys {
			if let Some(s) = obj.get(*key).and_then(|v| v.as_str())
				&& !s.is_empty()
				&& !s.starts_with('{')
				&& !s.starts_with('[')
			{
				return Some((s.to_string(), "text".to_string()));
			}
		}
	}

	None
}

/// Store content in the session's inline store and record a catalog marker.
/// `center` is an optional caller-supplied string that travels with the
/// marker (schema param `_ccr_center`) - previously documented but silently
/// dropped (F11); now threaded through to both the recorded catalog entry
/// and the rendered marker.
/// Returns `{hash, type, size, preview, marker}`.
fn compress_into(state:&mut AphroditeState, content:&str, hint:&str, center:Option<&str>) -> serde_json::Value {
	// Hermes wraps tool results in JSON. Unwrap to find real content for
	// classification/preview (terminal output, diffs, etc.) instead of a
	// meaningless "[json:1items 1L]" - but always hash and store the
	// ORIGINAL `content`, never the extracted piece: retrieval must return
	// exactly what was passed in, including wrapper metadata like
	// `exit_code`/`error`, and legitimate caller JSON that merely matches a
	// wrapper shape (e.g. `{"content":"hi","id":42}`) must round-trip intact.
	let (classify_content, eff_type) = if let Some((c, t)) = unwrap_hermes_result(content) {
		// ISSUE-11 WS1 (caller-hint-wins): the caller's explicit `type`
		// hint is intent, the unwrap is a guess - when a non-default hint
		// is present it must win. For JSON payloads the preview is built
		// from the FULL content: an unwrap fragment ("ok") produces a
		// wrong `[text:1L 2B | ok]` that counts the fragment, not the
		// stored payload, and drops the shape signal entirely.
		if !hint.is_empty() && hint != "text" {
			if content.trim_start().starts_with('{') {
				(content.to_string(), hint.to_string())
			} else {
				(c, hint.to_string())
			}
		} else {
			(c, t)
		}
	} else {
		let detected = aphrodite::detect_type(content);
		let ccr_type = if hint.is_empty() || hint == "text" { detected } else { hint.to_string() };
		(content.to_string(), ccr_type)
	};

	let hash = aphrodite::hooks::compute_hash(content);
	let preview = aphrodite::build_preview(&eff_type, &classify_content);
	let marker = aphrodite::marker::ccr_marker(&hash, &eff_type, content.len(), &preview, None, None, center);

	state.inline_store_put(hash.clone(), content.to_string());
	state.record_marker(MarkerEntry {
		hash:hash.clone(),
		ccr_type:eff_type.clone(),
		size:content.len(),
		preview:preview.clone(),
		turn:state.turn_counter,
		center:center.map(|c| c.to_string()),
		meta:None,
	});

	serde_json::json!({
		"hash": hash,
		"type": eff_type,
		"size": content.len(),
		"preview": preview,
		"marker": marker,
	})
}

fn tool_registry() -> HashMap<&'static str, ToolHandler> {
	let mut m:HashMap<&'static str, ToolHandler> = HashMap::new();

	// ── compress: store content, return a resolvable CCR marker ──
	m.insert("aphrodite_compress", |args| {
		let content = str_arg(args, "content");
		if content.is_empty() {
			return serde_json::json!({"error": "content is required"});
		}
		let hint = str_arg(args, "type");
		let center = args.get("_ccr_center").and_then(|v| v.as_str());
		with_shared(|state| compress_into(state, content, hint, center))
	});

	// ── retrieve: resolve a CCR hash (recursively) or read a path directly ──
	m.insert("aphrodite_retrieve", |args| {
		let path = str_arg(args, "path");
		if !path.is_empty() {
			return match read_path_guarded(path) {
				Ok(content) => {
					let query = str_arg(args, "query");
					let body = if query.is_empty() {
						content
					} else {
						aphrodite::resolve::filter_lines(&content, query)
					};
					serde_json::json!({"found": true, "source": "path", "path": path, "content": body})
				},
				Err(e) => serde_json::json!({"found": false, "error": e}),
			};
		}

		let hash = str_arg(args, "hash");
		if hash.is_empty() {
			return serde_json::json!({"error": "hash or path is required"});
		}
		let query = str_arg(args, "query").to_string();
		with_shared(|state| {
			match aphrodite::resolve::expand(state, hash) {
				Some(content) => {
					let body = if query.is_empty() {
						content
					} else {
						aphrodite::resolve::filter_lines(&content, &query)
					};
					serde_json::json!({"found": true, "source": "ccr", "hash": hash, "content": body})
				},
				None => serde_json::json!({"found": false, "hash": hash, "error": "hash not found in session store"}),
			}
		})
	});

	// ── stats: live session + proxy health ──
	m.insert("aphrodite_stats", |_args| {
		let mut stats = with_shared(|state| {
			serde_json::json!({
				"version": env!("CARGO_PKG_VERSION"),
				"engine": "aphrodite-hermes",
				"inline_entries": state.inline_store.len(),
				"markers": state.recent_markers.len(),
				"referenced_files": state.referenced_files.len(),
				"archived_turns": state.conv_index.len(),
				"turn": state.turn_counter,
				"engine_enabled": state.context_engine_enabled,
				"threshold_pct": state.engine_threshold_pct,
				"tool_threshold": state.tool_threshold,
				"terminal_threshold": state.terminal_threshold,
				"auto_reload": state.auto_reload,
				// Config self-diagnosis: present when aphrodite.toml was
				// found but failed to parse - defaults are in effect.
				"config_error": state.config_error,
				// Tier 1 teaching loop telemetry: the adaptive split
				// threshold and the consequence ledger. Diagnostics only -
				// never rendered into the LLM's conversational view.
				"chain_split_min_segments": state.chain_split_min_segments,
				"chain_split_events": state.split_events.len(),
				"chain_split_produced": state.split_events.iter().map(|e| e.produced).sum::<usize>(),
				"chain_split_retrieved": state.split_events.iter().map(|e| e.retrieved).sum::<usize>(),
			})
		});
		stats["proxies"] = proxy_health();
		stats
	});

	// ── catalog: full or table-of-contents view of recorded markers ──
	m.insert("aphrodite_catalog", |args| {
		let mode = {
			let m = str_arg(args, "mode");
			if m.is_empty() { "full" } else { m }
		};
		with_shared(|state| {
			let items:Vec<serde_json::Value> = state
				.recent_markers
				.iter()
				.rev()
				.map(|e| {
					if mode == "toc" {
						serde_json::json!({
							"hash": &e.hash,
							"type": e.ccr_type, "size": e.size, "preview": e.preview,
						})
					} else {
						serde_json::json!({
							"hash": e.hash, "type": e.ccr_type, "size": e.size,
							"preview": e.preview, "turn": e.turn, "center": e.center,
						})
					}
				})
				.collect();
			serde_json::json!({"mode": mode, "total": items.len(), "items": items, "turn": state.turn_counter})
		})
	});

	// ── search: filter recorded markers by keyword and/or type ──
	m.insert("aphrodite_search", |args| {
		let query = str_arg(args, "query").to_lowercase();
		let type_filter = args.get("type").and_then(|v| v.as_str());
		with_shared(|state| {
			let results:Vec<serde_json::Value> = state
				.recent_markers
				.iter()
				.rev()
				.filter(|mk| {
					let q_ok = query.is_empty()
						|| mk.preview.to_lowercase().contains(&query)
						|| mk.ccr_type.to_lowercase().contains(&query);
					let t_ok = type_filter.is_none_or(|t| mk.ccr_type == t);
					q_ok && t_ok
				})
				.take(20)
				.map(|mk| {
					serde_json::json!({
						"hash": &mk.hash,
						"type": mk.ccr_type, "size": mk.size, "preview": mk.preview,
					})
				})
				.collect();
			serde_json::json!({"query": query, "total": results.len(), "results": results})
		})
	});

	// ── diff: conversation turn history (archived compressions) ──
	m.insert("aphrodite_diff", |_args| {
		with_shared(|state| {
			let turns = aphrodite::session::get_conv_index(state);
			serde_json::json!({"total": turns.len(), "turns": turns})
		})
	});

	// ── directive: list/swap/add/load/remove/reset active behavioral directives ──
	// (01-F3) - delegates to the same `directives::handle_action` the core
	// crate's `aphrodite_directive`/`aphrodite_dispatch` C ABI entry points
	// use, so this bridge exposes the identical action set/error shape.
	m.insert("aphrodite_directive", |args| {
		let action = str_arg(args, "action");
		let action = if action.is_empty() { "list" } else { action };
		let name = str_arg(args, "name");
		with_shared(|state| aphrodite::directives::handle_action(state, action, name))
	});

	// ── files: file paths referenced this session ──
	m.insert("aphrodite_files", |_args| {
		with_shared(|state| {
			let files:Vec<serde_json::Value> = state
				.referenced_files
				.iter()
				.map(|(path, tool)| serde_json::json!({"path": path, "tool": tool}))
				.collect();
			serde_json::json!({"total": files.len(), "files": files})
		})
	});

	// ── prefetch: read + compress files now, return markers ──
	m.insert("aphrodite_prefetch", |args| {
		let paths:Vec<String> = args
			.get("paths")
			.and_then(|v| v.as_array())
			.map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
			.unwrap_or_default();
		if paths.is_empty() {
			return serde_json::json!({"error": "paths is required"});
		}
		// F4/T5 (report 06): read files from disk BEFORE taking the
		// process-global `with_shared` lock, not inside it - this crate's
		// `STATE` is a single `Mutex` shared by every hook and tool call, so
		// disk I/O held under it (network/USB-mounted paths, slow filesystems)
		// stalls every other concurrent hook/tool for the duration of the read.
		let outcomes = aphrodite::prefetch::read_paths(&paths);
		with_shared(|state| aphrodite::prefetch::insert_outcomes(state, outcomes))
	});

	// ── prefetch_status: which prefetched files are resolvable ──
	// Prefetch is synchronous here, so anything loaded is already "ready".
	m.insert("aphrodite_prefetch_status", |_args| {
		with_shared(|state| {
			let ready:Vec<serde_json::Value> = state
				.recent_markers
				.iter()
				.filter_map(|mk| {
					mk.meta.as_ref().and_then(|meta| meta.get("path")).map(|path| {
						serde_json::json!({
							"path": path,
							"hash": &mk.hash,
							"type": mk.ccr_type, "size": mk.size,
						})
					})
				})
				.collect();
			serde_json::json!({"loading": [], "ready": ready, "errors": [], "total_ready": ready.len()})
		})
	});

	// ── reclassify: re-detect type/preview for stored markers ──
	m.insert("aphrodite_reclassify", |args| {
		let only_hash = args.get("hash").and_then(|v| v.as_str());
		with_shared(|state| {
			// Collect (hash, fresh content) for markers we will touch, then
			// recompute type + preview from the stored content.
			let targets:Vec<String> = state
				.recent_markers
				.iter()
				.filter(|mk| only_hash.is_none_or(|h| mk.hash == h))
				.map(|mk| mk.hash.clone())
				.collect();

			let mut updated = 0usize;
			for hash in targets {
				if let Some(content) = state.inline_store_get(&hash) {
					let detected = aphrodite::detect_type(&content);
					let preview = aphrodite::build_preview(&detected, &content);
					if let Some(mk) = state.recent_markers.iter_mut().find(|m| m.hash == hash) {
						mk.ccr_type = detected;
						mk.preview = preview;
						updated += 1;
					}
				}
			}
			serde_json::json!({"status": "ok", "reclassified": updated})
		})
	});

	// ── test: in-process smoke test of the compress → retrieve loop ──
	m.insert("aphrodite_test", |args| {
		let mode = {
			let m = str_arg(args, "mode");
			if m.is_empty() { "quick" } else { m }
		};
		let samples:&[(&str, &str)] = match mode {
			"quick" => &[("fn main() { println!(\"hi\"); }\n", "source_code")],
			_ => {
				&[
					("fn main() { println!(\"hi\"); }\n", "source_code"),
					("error[E0382]: borrow of moved value\nwarning: unused\n", "build"),
					("{\"a\":1,\"b\":2,\"c\":3}\n", "json_array"),
				]
			},
		};
		let mut checks = Vec::new();
		let mut passed = 0usize;
		with_shared(|state| {
			for (content, hint) in samples {
				let info = compress_into(state, content, hint, None);
				let hash = info["hash"].as_str().unwrap_or("");
				let round = aphrodite::resolve::expand(state, hash);
				let ok = round.as_deref() == Some(*content);
				if ok {
					passed += 1;
				}
				checks.push(serde_json::json!({
					"type": hint, "hash": hash,
					"roundtrip": ok,
				}));
			}
		});
		serde_json::json!({
			"mode": mode,
			"status": if passed == checks.len() { "ok" } else { "fail" },
			"passed": passed,
			"total": checks.len(),
			"checks": checks,
			"proxies": proxy_health(),
		})
	});

	// ── rebuild: operational helper - reports binary + proxy state ──
	// The dylib cannot safely rebuild itself mid-session; surface the state the
	// operator needs and let the standalone proxy / dev loop do the rebuild.
	m.insert("aphrodite_rebuild", |_args| {
		serde_json::json!({
			"status": "ok",
			"version": env!("CARGO_PKG_VERSION"),
			"proxies": proxy_health(),
			"hint": "rebuild via `cargo build --release -p aphrodite`; restart the session to pick up the new dylib",
		})
	});

	// ── debug: per-session debug toggle (Rust-side only) ──
	// Dev-only: gated behind debug_assertions so release dylibs register
	// exactly the 13 production tools (catalog validate rule 6 - the
	// manifest declares 13; a release dylib registering aphrodite_debug
	// fails `hermes plugins validate` as an undeclared tool). Dev builds
	// (cargo watch / debug profile) keep it; `cargo build --release` drops
	// it. The debug-prefix/session-record machinery in debug.rs remains
	// live in both profiles (it only activates on a flag file).
	// Flips the session-scoped flag file in the runtime home. Resolution is
	// root-session based (see debug.rs): the CURRENT session's root id keys the
	// flag, so subagents inherit it and other sessions stay quiet. Pure Rust -
	// the customer-facing Python shim is untouched.
	#[cfg(debug_assertions)]
	m.insert("aphrodite_debug", |args| {
		let on = args.get("on").and_then(|v| v.as_bool()).unwrap_or(true);
		match crate::debug::set_enabled_current(on) {
			Ok((session, flag)) => {
				serde_json::json!({
					"status": "ok",
					"debug": on,
					"session": session,
					"flag": flag,
				})
			},
			Err(e) => serde_json::json!({"error": e}),
		}
	});

	// ── context engine pre-LLM hook (registered via ctx.register_context_engine) ──
	// 05-P1/T1: same single assembler as the bridge `pre_llm_call` arm and core
	// `hooks::pre_llm_call`, so this path can't fork on what the model sees.
	m.insert("context_engine_pre_llm", |_args| {
		with_shared(|state| {
			let context = aphrodite::flow::build_turn_context(state, None);
			if context.is_empty() {
				serde_json::Value::Null
			} else {
				serde_json::json!({"context": context})
			}
		})
	});

	m
}

#[cfg(test)]
mod tests {
	use super::*;

	// ── 01-F6/F12: table-driven coverage of every `unwrap_hermes_result`
	// branch - this ~100-line heuristic had zero regression tests despite
	// being rewritten three times (bf181d7 -> 9e52762 -> 8f138c1).
	/// Table row: label, input value, expected (status, kind).
	type UnwrapCase<'a> = (&'a str, serde_json::Value, Option<(&'a str, &'a str)>);

	#[test]
	fn test_unwrap_hermes_result_table() {
		let cases:Vec<UnwrapCase<'_>> = vec![
			(
				"terminal output+exit_code",
				serde_json::json!({"output": "hello\n", "exit_code": 0}),
				Some(("hello\n", "text")),
			),
			(
				"terminal with exit code marker",
				serde_json::json!({"output": "boom\nexit code: 1\n", "exit_code": 1}),
				Some(("boom\nexit code: 1\n", "terminal")),
			),
			(
				"empty output falls through to error (F12)",
				serde_json::json!({"output": "", "exit_code": 1, "error": "command not found: cargp"}),
				Some(("command not found: cargp", "text")),
			),
			(
				"empty output, no error, no success -> None",
				serde_json::json!({"output": "", "exit_code": 1}),
				None,
			),
			(
				"diff",
				serde_json::json!({"success": true, "diff": "-a\n+b\n"}),
				Some(("-a\n+b\n", "text")),
			),
			(
				"error string",
				serde_json::json!({"error": "Found 3 matches"}),
				Some(("Found 3 matches", "text")),
			),
			(
				"success bool only -> None (no collapse)",
				serde_json::json!({"success": true}),
				None,
			),
			(
				"success bool with data payload",
				serde_json::json!({"success": true, "data": {"web": [{"title": "x"}]}}),
				None,
			),
			(
				"success string with data payload",
				serde_json::json!({"success": "ok", "data": [1]}),
				None,
			),
			(
				"success string message",
				serde_json::json!({"success": "wrote 3 files"}),
				Some(("wrote 3 files", "text")),
			),
			(
				"search with matches",
				serde_json::json!({"total_count": 1, "matches": [{"path": "a.rs", "line": 3, "content": "fn x()"}]}),
				Some(("a.rs:3:fn x()", "search")),
			),
			(
				"search with matches_text (real Hermes shape)",
				serde_json::json!({
					"total_count": 19,
					"matches_format": "path-grouped",
					"matches_text": "src/a.rs\n  10: fn alpha()\nsrc/b.rs\n  3: struct Gamma\n",
				}),
				Some(("src/a.rs:10:fn alpha()\nsrc/b.rs:3:struct Gamma", "search")),
			),
			(
				"search without matches surfaces total as text",
				serde_json::json!({"total_count": 5, "truncated": true}),
				Some(("5 total (truncated)", "text")),
			),
			(
				"data object with total_count key only -> None",
				serde_json::json!({"total_count": 42, "items": [1, 2, 3]}),
				None,
			),
			(
				"file read content",
				serde_json::json!({"content": "hi", "total_lines": 1}),
				Some(("hi", "text")),
			),
			(
				"priority key single-key",
				serde_json::json!({"description": "does a thing"}),
				Some(("does a thing", "text")),
			),
			(
				"priority key multi-key -> None (no collapse)",
				serde_json::json!({"name": "skill", "description": "does a thing"}),
				None,
			),
			(
				"priority key multi-key data payload",
				serde_json::json!({"result": "ok", "data": {"web": [{"title": "x"}]}}),
				None,
			),
			("not an object", serde_json::json!(["a", "b"]), None),
			("plain string, not JSON", serde_json::json!("hello"), None),
			("no matching shape", serde_json::json!({"a": 1, "b": 2}), None),
		];

		for (label, input, expected) in cases {
			let content = input.to_string();
			let got = unwrap_hermes_result(&content);
			match expected {
				Some((c, t)) => {
					let (gc, gt) = got.unwrap_or_else(|| panic!("{label}: expected Some, got None"));
					assert_eq!(gc, c, "{label}: content mismatch");
					assert_eq!(gt, t, "{label}: type mismatch");
				},
				None => assert!(got.is_none(), "{label}: expected None, got {got:?}"),
			}
		}
	}

	#[test]
	fn test_compress_then_retrieve_roundtrip() {
		// The core promise: a tool can compress content and retrieve it back
		// via a separate dispatch call, because state is shared across calls.
		let _g = crate::test_guard();
		let content = "fn answer() -> i32 { 42 }\n".repeat(10);
		let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
		let hash = compressed["hash"].as_str().expect("hash present").to_string();
		assert!(!hash.is_empty());

		let retrieved = dispatch("aphrodite_retrieve", &serde_json::json!({"hash": hash}).to_string());
		assert_eq!(retrieved["found"], true, "retrieve must resolve a just-compressed hash");
		assert_eq!(retrieved["content"], content);
	}

	// ── 01-F2: `unwrap_hermes_result` is a heuristic used to pick a better
	// type/preview - it must never change what a retrieve returns. Caller
	// JSON that merely happens to match a wrapper shape (a "content" key)
	// must still round-trip byte-for-byte, not collapse to the extracted
	// field alone.
	#[test]
	fn test_compress_preserves_original_when_content_looks_like_a_wrapper() {
		let _g = crate::test_guard();
		let content = serde_json::json!({"content": "hi", "id": 42, "role": "assistant"}).to_string();
		let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
		let hash = compressed["hash"].as_str().expect("hash present").to_string();

		let retrieved = dispatch("aphrodite_retrieve", &serde_json::json!({"hash": hash}).to_string());
		assert_eq!(retrieved["found"], true);
		assert_eq!(
			retrieved["content"], content,
			"retrieve must return the exact original JSON, not just the extracted \"content\" field"
		);
	}

	// ── 01-F2: a genuine terminal wrapper must also round-trip losslessly -
	// `exit_code`/`error` are exactly the fields most likely to matter for a
	// failed command, and must still be recoverable after compression.
	#[test]
	fn test_compress_preserves_terminal_wrapper_metadata() {
		let _g = crate::test_guard();
		let content = serde_json::json!({"output": "error: broke\nexit code: 1\n", "exit_code": 1}).to_string();
		let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
		let hash = compressed["hash"].as_str().expect("hash present").to_string();
		assert_eq!(compressed["type"], "terminal", "type should reflect the unwrapped payload");

		let retrieved = dispatch("aphrodite_retrieve", &serde_json::json!({"hash": hash}).to_string());
		assert_eq!(retrieved["found"], true);
		assert_eq!(
			retrieved["content"], content,
			"retrieve must return the original wrapper, including exit_code, not just the extracted output"
		);
	}

	// ── ISSUE-11 (WS1 + WS2-tools): the JSON-preview collapse family.
	// A JSON tool payload like {"success":true,"data":{...}} must never
	// collapse to the literal fragment "ok" (`[text:1L 2B | ok]`); the
	// caller's explicit `type` hint must win over the envelope heuristic,
	// and search results must show the REAL match count. ──

	// ── FIXDESIGN 3.1: the issue's exact repro shape, explicit path.
	// {"success":true,"data":{...}} with type:"tool_result" must keep the
	// hinted type and preview the FULL payload (honest L/B), never
	// "[text:1L 2B | ok]". ──
	#[test]
	fn test_compress_json_payload_with_hint_keeps_type_and_full_preview() {
		let _g = crate::test_guard();
		let content = serde_json::json!({"success": true, "data": {"web": [{"title": "x"}]}}).to_string();
		let compressed = dispatch(
			"aphrodite_compress",
			&serde_json::json!({"content": content, "type": "tool_result"}).to_string(),
		);
		assert_eq!(
			compressed["type"], "tool_result",
			"explicit hint must win over the envelope heuristic"
		);
		let preview = compressed["preview"].as_str().unwrap();
		assert!(
			preview.starts_with("[tool_result:"),
			"preview must reflect the hinted type: {preview}"
		);
		assert!(
			!preview.contains("| ok]"),
			"preview must not collapse to the literal 'ok' fragment: {preview}"
		);
		assert!(
			preview.contains("\"success\""),
			"preview must show the real JSON payload: {preview}"
		);
		assert_eq!(
			compressed["size"],
			content.len(),
			"size must count the full payload, not the fragment"
		);
		// Round-trip stays lossless regardless of preview/type.
		let retrieved = dispatch(
			"aphrodite_retrieve",
			&serde_json::json!({"hash": compressed["hash"]}).to_string(),
		);
		assert_eq!(retrieved["content"], content);
	}

	// ── FIXDESIGN 3.2 (adapted): no-hint bare {"success":true} must not
	// collapse either - the preview must show the payload itself, never a
	// 2-byte "ok" fragment. (Note: headroom classifies JSON *objects* as
	// text - only arrays get json_array - so we assert preview honesty,
	// not a json type, which the classifier cannot produce here.) ──
	#[test]
	fn test_compress_bare_success_object_preview_is_honest() {
		let _g = crate::test_guard();
		let content = serde_json::json!({"success": true}).to_string();
		let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
		let preview = compressed["preview"].as_str().unwrap();
		assert!(
			!preview.contains("| ok]"),
			"bare success object must not collapse to 'ok': {preview}"
		);
		assert!(preview.contains("\"success\""), "preview must show the real payload: {preview}");
		assert_ne!(
			compressed["type"], "search",
			"the unwrap must not hijack a success payload into search"
		);
		assert_eq!(compressed["size"], content.len());
	}

	// ── WS2-tools: the real Hermes search_files shape ships `matches_text`
	// (path-grouped) instead of a `matches` array. The preview must count
	// the REAL matches, not collapse to "[search:1L]". ──
	#[test]
	fn test_compress_search_result_shows_real_match_count() {
		let _g = crate::test_guard();
		let content = serde_json::json!({
			"total_count": 19,
			"matches_format": "path-grouped",
			"matches_text": "src/a.rs\n  10: fn alpha()\nsrc/b.rs\n  3: struct Gamma\n",
		})
		.to_string();
		let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
		assert_eq!(compressed["type"], "search", "search-shaped envelope must classify as search");
		let preview = compressed["preview"].as_str().unwrap();
		assert!(
			preview.starts_with("[search:2 hits in 2 files"),
			"preview must count the real matches: {preview}"
		);
		assert!(
			!preview.contains("[search:1L]"),
			"no [search:1L] collapse for a real search: {preview}"
		);
	}

	// ── WS2-tools: the take(20) cap is gone - a matches array larger than
	// 20 must not preview as "[search:20 hits ...]". ──
	#[test]
	fn test_compress_search_matches_array_counts_all_matches() {
		let _g = crate::test_guard();
		let matches:Vec<serde_json::Value> = (0..25)
			.map(|i| serde_json::json!({"path": format!("f{i}.rs"), "line": i, "content": "x"}))
			.collect();
		let content = serde_json::json!({"total_count": 25, "matches": matches}).to_string();
		let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
		let preview = compressed["preview"].as_str().unwrap();
		assert!(
			preview.contains("25 hits"),
			"all real matches must be counted, not capped at 20: {preview}"
		);
	}

	// ── WS2-tools: a zero-hit / count-only search must surface the REAL
	// total ("0 total") instead of an unreadable "[search:1L]". ──
	#[test]
	fn test_compress_search_zero_or_count_only_surfaces_total() {
		let _g = crate::test_guard();
		let content = serde_json::json!({"total_count": 0, "matches": []}).to_string();
		let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
		let preview = compressed["preview"].as_str().unwrap();
		assert!(
			preview.contains("0 total"),
			"zero-hit search must show the real total: {preview}"
		);
		assert!(
			!preview.contains("[search:"),
			"count-only search must not render [search:1L]: {preview}"
		);
	}

	// ── WS2-tools: a data object that merely carries a total_count key
	// (no matches/matches_text/matches_format/truncated) is NOT a search
	// envelope - it must preview as itself, not as a fake search. ──
	#[test]
	fn test_compress_data_object_with_total_count_key_is_not_search() {
		let _g = crate::test_guard();
		let content = serde_json::json!({"total_count": 42, "items": [1, 2, 3]}).to_string();
		let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
		assert_ne!(
			compressed["type"], "search",
			"total_count alone must not hijack the type: {compressed}"
		);
		let preview = compressed["preview"].as_str().unwrap();
		assert!(preview.contains("items"), "preview must show the real payload: {preview}");
	}

	// ── T5 (F3): the "aphrodite_retrieve" tool delegates to
	// `aphrodite::resolve::expand` -> `resolve_one`, which already strips a
	// `|type|size` marker-body suffix and surrounding whitespace - this
	// pins that the delegation actually carries the tolerance through.
	#[test]
	fn test_retrieve_normalizes_pipe_suffixed_and_whitespace_hash() {
		let _g = crate::test_guard();
		let content = "fn answer() -> i32 { 42 }\n".repeat(10);
		let compressed = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
		let hash = compressed["hash"].as_str().expect("hash present").to_string();

		for hash_arg in [hash.clone(), format!("{hash}|tool|1024"), format!("  {hash}  ")] {
			let retrieved = dispatch("aphrodite_retrieve", &serde_json::json!({"hash": hash_arg}).to_string());
			assert_eq!(retrieved["found"], true, "hash arg {hash_arg:?} must resolve: {retrieved:?}");
			assert_eq!(retrieved["content"], content);
		}
	}

	// ── 01-F3: `aphrodite_directive` (the bridge tool, not the core C ABI
	// export) must be dispatchable and delegate to the same
	// `directives::handle_action` as everything else - list/swap round-trip
	// through the shared session state. ──
	#[test]
	fn test_aphrodite_directive_tool_list_and_swap() {
		let _g = crate::test_guard();
		with_shared(|state| {
			// `active_directives` is process-global and NOT reset between
			// tests (deliberately - directives persist across a session
			// reset just like the inline store); start from a known-empty
			// state rather than assuming one.
			state.active_directives.clear();
			state.directives.insert(
				"focus".into(),
				aphrodite::directives::Directive { name:"focus".into(), content:"stay focused".into() },
			);
		});

		let listed = dispatch("aphrodite_directive", "{}");
		assert_eq!(listed["available"], serde_json::json!(["focus"]));
		assert_eq!(listed["active"], serde_json::json!([]));

		let swapped = dispatch(
			"aphrodite_directive",
			&serde_json::json!({"action": "swap", "name": "focus"}).to_string(),
		);
		assert_eq!(swapped["swapped"], "focus");
		assert_eq!(swapped["active"], serde_json::json!(["focus"]));
	}

	// ── T9 (F11): `_ccr_center` was documented in the schema but silently
	// dropped by compress_into; it must now travel with the recorded marker.
	#[test]
	fn test_compress_wires_ccr_center_through() {
		let _g = crate::test_guard();
		let content = "fn answer() -> i32 { 42 }\n".repeat(10);
		let compressed = dispatch(
			"aphrodite_compress",
			&serde_json::json!({"content": content, "_ccr_center": "my-center"}).to_string(),
		);
		let hash = compressed["hash"].as_str().expect("hash present").to_string();
		let catalog = dispatch("aphrodite_catalog", "{}");
		let entry = catalog["items"]
			.as_array()
			.unwrap()
			.iter()
			.find(|e| e["hash"] == hash)
			.expect("just-compressed entry should be in the catalog");
		assert_eq!(entry["center"], "my-center");
	}

	#[test]
	fn test_retrieve_with_query_filters_lines() {
		let _g = crate::test_guard();
		let content = "alpha line\nbeta error here\ngamma line\n";
		let c = dispatch("aphrodite_compress", &serde_json::json!({"content": content}).to_string());
		let hash = c["hash"].as_str().unwrap().to_string();
		let r = dispatch(
			"aphrodite_retrieve",
			&serde_json::json!({"hash": hash, "query": "error"}).to_string(),
		);
		let body = r["content"].as_str().unwrap();
		assert!(body.contains("beta error here"));
		assert!(!body.contains("alpha line"));
	}

	#[test]
	fn test_retrieve_path_outside_workspace_is_denied() {
		// /etc/hosts exists and is outside any repo cwd → must be refused.
		let _g = crate::test_guard();
		let r = dispatch("aphrodite_retrieve", &serde_json::json!({"path": "/etc/hosts"}).to_string());
		assert_eq!(r["found"], false, "reads outside the workspace must be denied: {r:?}");
		assert!(r["error"].as_str().unwrap().contains("outside the workspace"));
	}

	#[test]
	fn test_retrieve_path_within_workspace_ok() {
		// This source file is inside the crate (workspace) → allowed.
		let _g = crate::test_guard();
		let r = dispatch(
			"aphrodite_retrieve",
			&serde_json::json!({"path": concat!(env!("CARGO_MANIFEST_DIR"), "/src/tools.rs")}).to_string(),
		);
		assert_eq!(r["found"], true, "in-workspace path read should succeed: {r:?}");
		assert!(r["content"].as_str().unwrap().contains("read_path_guarded"));
	}

	#[test]
	fn test_retrieve_missing_hash() {
		let _g = crate::test_guard();
		let r = dispatch(
			"aphrodite_retrieve",
			&serde_json::json!({"hash": "deadbeefdeadbeefdeadbeef"}).to_string(),
		);
		assert_eq!(r["found"], false);
	}

	#[test]
	fn test_catalog_and_search_see_compressions() {
		let _g = crate::test_guard();
		// Reset shared state so `total` reflects only this test's compression,
		// making the previously-loosened `>= 1` assertions exact.
		crate::with_shared(aphrodite::session::on_session_start);
		dispatch(
			"aphrodite_compress",
			&serde_json::json!({"content": "needle_xyz in a haystack\n".repeat(5), "type": "log"}).to_string(),
		);
		let cat = dispatch("aphrodite_catalog", "{}");
		assert_eq!(cat["total"].as_u64().unwrap(), 1);
		let found = dispatch("aphrodite_search", &serde_json::json!({"query": "log"}).to_string());
		assert_eq!(found["total"].as_u64().unwrap(), 1);
	}

	#[test]
	fn test_test_tool_roundtrips() {
		let _g = crate::test_guard();
		let r = dispatch("aphrodite_test", &serde_json::json!({"mode": "full"}).to_string());
		assert_eq!(r["status"], "ok", "smoke test should pass: {:?}", r);
		assert_eq!(r["passed"], r["total"]);
	}

	// ── Config self-diagnosis: a parse-failure recorded on the shared
	// state must surface verbatim in `aphrodite_stats` (the tracing warn
	// is a no-op in the Hermes dylib, so this field is the only
	// guaranteed-visible signal that defaults are in effect). ──
	#[test]
	fn test_stats_surfaces_config_error() {
		let _g = crate::test_guard();
		let broken_path = "/tmp/aphrodite-cfg-broken-test.toml";

		// Simulate a found-but-broken aphrodite.toml (what
		// `Config::load().apply_compression()` records on the state).
		with_shared(|s| s.config_error = Some(format!("{broken_path}: TOML parse error")));

		let r = dispatch("aphrodite_stats", "{}");
		assert_eq!(r["config_error"].as_str().unwrap(), format!("{broken_path}: TOML parse error"));
		assert!(
			r["auto_reload"].as_bool().unwrap_or(true) == false,
			"auto_reload must surface (default off): {:?}",
			r["auto_reload"]
		);

		// Restore the shared state so later tests are hermetic.
		with_shared(|s| s.config_error = None);
		let r2 = dispatch("aphrodite_stats", "{}");
		assert!(r2["config_error"].is_null(), "config_error must be absent when config parsed fine");
	}

	#[test]
	fn test_unknown_tool() {
		let r = dispatch("nonexistent", "{}");
		assert!(r["error"].as_str().unwrap().contains("unknown tool"));
	}

	// ── T11: schema/registry consistency ──────────────────────────
	#[test]
	fn schema_registry_names_match_tool_registry_keys() {
		// Catches classic registry/schema drift: every dispatchable tool must
		// have a schema, and vice versa (except `context_engine_pre_llm`,
		// which is an internal hook handler registered via
		// `ctx.register_context_engine` - never exposed as a callable tool
		// to Hermes, so it has no schema by design).
		let registry_names:std::collections::HashSet<&str> = tool_registry()
			.keys()
			.copied()
			.filter(|n| *n != "context_engine_pre_llm")
			.collect();
		let schema_names:std::collections::HashSet<String> = crate::schemas::all_schemas()
			.iter()
			.map(|s| s["name"].as_str().unwrap().to_string())
			.collect();

		for name in &registry_names {
			assert!(
				schema_names.contains(*name),
				"tool {name:?} is dispatchable but has no schema in schemas::all_schemas()"
			);
		}
		for name in &schema_names {
			assert!(
				registry_names.contains(name.as_str()),
				"schema {name:?} exists but is not dispatchable via tool_registry()"
			);
		}
	}

	#[test]
	fn test_prefetch_real_file() {
		let _g = crate::test_guard();
		let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src/tools.rs");
		let r = dispatch("aphrodite_prefetch", &serde_json::json!({"paths": [src]}).to_string());
		assert_eq!(r["loaded"], 1, "prefetch should load this source file: {:?}", r);
	}
}
