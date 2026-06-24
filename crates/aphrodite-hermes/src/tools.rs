//! Tool dispatch - routes Hermes tool calls to aphrodite core functions.
//!
//! Each Hermes tool (`aphrodite_compress`, `aphrodite_retrieve`, ...) has a
//! handler here that parses args, operates on the process-global session state
//! (see [`crate::with_shared`]), and returns a JSON result. Because every
//! handler shares one state, content compressed by a hook or `aphrodite_compress`
//! stays resolvable by `aphrodite_retrieve` for the life of the session.

use std::collections::HashMap;

use aphrodite::state::{AphroditeState, MarkerEntry};

use crate::{proxy_health, with_shared};

type ToolHandler = fn(args: &serde_json::Value) -> serde_json::Value;

/// Dispatch a tool by name. Returns `{"error": "..."}` for unknown tools.
pub fn dispatch(name: &str, args_json: &str) -> serde_json::Value {
    let registry = tool_registry();
    match registry.get(name) {
        Some(handler) => {
            let args: serde_json::Value = match serde_json::from_str(args_json) {
                Ok(v) => v,
                Err(e) => return serde_json::json!({"error": format!("invalid args: {}", e)}),
            };
            handler(&args)
        },
        None => serde_json::json!({"error": format!("unknown tool: {}", name)}),
    }
}

// ── Shared helpers ─────────────────────────────────────────

fn str_arg<'a>(args: &'a serde_json::Value, key: &str) -> &'a str {
    args.get(key).and_then(|v| v.as_str()).unwrap_or("")
}

/// Store content in the session's inline store and record a catalog marker.
/// Returns `{hash, type, size, preview, marker}`.
fn compress_into(state: &mut AphroditeState, content: &str, hint: &str) -> serde_json::Value {
    let detected = aphrodite::detect_type(content);
    let ccr_type = if hint.is_empty() || hint == "text" {
        detected
    } else {
        hint.to_string()
    };
    let hash = aphrodite::hooks::compute_hash(content);
    let preview = aphrodite::build_preview(&ccr_type, content);
    let marker =
        aphrodite::marker::ccr_marker(&hash, &ccr_type, content.len(), &preview, None, None, None);

    state.inline_store_put(hash.clone(), content.to_string());
    state.record_marker(MarkerEntry {
        hash: hash.clone(),
        ccr_type: ccr_type.clone(),
        size: content.len(),
        preview: preview.clone(),
        turn: state.turn_counter,
        center: None,
        meta: None,
    });

    serde_json::json!({
        "hash": hash,
        "type": ccr_type,
        "size": content.len(),
        "preview": preview,
        "marker": marker,
    })
}

fn tool_registry() -> HashMap<&'static str, ToolHandler> {
    let mut m: HashMap<&'static str, ToolHandler> = HashMap::new();

    // ── compress: store content, return a resolvable CCR marker ──
    m.insert("aphrodite_compress", |args| {
        let content = str_arg(args, "content");
        if content.is_empty() {
            return serde_json::json!({"error": "content is required"});
        }
        let hint = str_arg(args, "type");
        with_shared(|state| compress_into(state, content, hint))
    });

    // ── retrieve: resolve a CCR hash (recursively) or read a path directly ──
    m.insert("aphrodite_retrieve", |args| {
        let path = str_arg(args, "path");
        if !path.is_empty() {
            return match std::fs::read_to_string(path) {
                Ok(content) => {
                    let query = str_arg(args, "query");
                    let body = if query.is_empty() {
                        content
                    } else {
                        aphrodite::resolve::filter_lines(&content, query)
                    };
                    serde_json::json!({"found": true, "source": "path", "path": path, "content": body})
                }
                Err(e) => serde_json::json!({"found": false, "error": format!("read {}: {}", path, e)}),
            };
        }

        let hash = str_arg(args, "hash");
        if hash.is_empty() {
            return serde_json::json!({"error": "hash or path is required"});
        }
        let query = str_arg(args, "query").to_string();
        with_shared(|state| match aphrodite::resolve::expand(state, hash) {
            Some(content) => {
                let body = if query.is_empty() {
                    content
                } else {
                    aphrodite::resolve::filter_lines(&content, &query)
                };
                serde_json::json!({"found": true, "source": "ccr", "hash": hash, "content": body})
            }
            None => serde_json::json!({"found": false, "hash": hash, "error": "hash not found in session store"}),
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
            let items: Vec<serde_json::Value> = state
                .recent_markers
                .iter()
                .rev()
                .map(|e| {
                    if mode == "toc" {
                        serde_json::json!({
                            "hash": &e.hash[..12.min(e.hash.len())],
                            "type": e.ccr_type, "size": e.size, "preview": e.preview,
                        })
                    } else {
                        serde_json::json!({
                            "hash": e.hash, "type": e.ccr_type, "size": e.size,
                            "preview": e.preview, "turn": e.turn,
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
            let results: Vec<serde_json::Value> = state
                .recent_markers
                .iter()
                .rev()
                .filter(|mk| {
                    let q_ok = query.is_empty()
                        || mk.preview.to_lowercase().contains(&query)
                        || mk.ccr_type.to_lowercase().contains(&query);
                    let t_ok = type_filter.map_or(true, |t| mk.ccr_type == t);
                    q_ok && t_ok
                })
                .take(20)
                .map(|mk| {
                    serde_json::json!({
                        "hash": &mk.hash[..12.min(mk.hash.len())],
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

    // ── files: file paths referenced this session ──
    m.insert("aphrodite_files", |_args| {
        with_shared(|state| {
            let files: Vec<serde_json::Value> = state
                .referenced_files
                .iter()
                .map(|(path, tool)| serde_json::json!({"path": path, "tool": tool}))
                .collect();
            serde_json::json!({"total": files.len(), "files": files})
        })
    });

    // ── prefetch: read + compress files now, return markers ──
    m.insert("aphrodite_prefetch", |args| {
        let paths: Vec<String> = args
            .get("paths")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        if paths.is_empty() {
            return serde_json::json!({"error": "paths is required"});
        }
        with_shared(|state| aphrodite::prefetch::prefetch_files(state, &paths))
    });

    // ── prefetch_status: which prefetched files are resolvable ──
    // Prefetch is synchronous here, so anything loaded is already "ready".
    m.insert("aphrodite_prefetch_status", |_args| {
        with_shared(|state| {
            let ready: Vec<serde_json::Value> = state
                .recent_markers
                .iter()
                .filter_map(|mk| {
                    mk.meta.as_ref().and_then(|meta| meta.get("path")).map(|path| {
                        serde_json::json!({
                            "path": path,
                            "hash": &mk.hash[..12.min(mk.hash.len())],
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
            let targets: Vec<String> = state
                .recent_markers
                .iter()
                .filter(|mk| only_hash.map_or(true, |h| mk.hash == h))
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
            if m.is_empty() {
                "quick"
            } else {
                m
            }
        };
        let samples: &[(&str, &str)] = match mode {
            "quick" => &[("fn main() { println!(\"hi\"); }\n", "source_code")],
            _ => &[
                ("fn main() { println!(\"hi\"); }\n", "source_code"),
                (
                    "error[E0382]: borrow of moved value\nwarning: unused\n",
                    "build",
                ),
                ("{\"a\":1,\"b\":2,\"c\":3}\n", "json_array"),
            ],
        };
        let mut checks = Vec::new();
        let mut passed = 0usize;
        with_shared(|state| {
            for (content, hint) in samples {
                let info = compress_into(state, content, hint);
                let hash = info["hash"].as_str().unwrap_or("");
                let round = aphrodite::resolve::expand(state, hash);
                let ok = round.as_deref() == Some(*content);
                if ok {
                    passed += 1;
                }
                checks.push(serde_json::json!({
                    "type": hint, "hash": &hash[..12.min(hash.len())],
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
            "hint": "rebuild via `cargo build --release -p aphrodite`; dylib hot-reloads on mtime change",
        })
    });

    // ── context engine pre-LLM hook (registered via ctx.register_context_engine) ──
    m.insert("context_engine_pre_llm", |_args| {
        with_shared(|state| {
            let summary = aphrodite::session::catalog_summary(state);
            if summary.is_empty() {
                serde_json::Value::Null
            } else {
                serde_json::json!({"context": summary})
            }
        })
    });

    m
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_then_retrieve_roundtrip() {
        // The core promise: a tool can compress content and retrieve it back
        // via a separate dispatch call, because state is shared across calls.
        let content = "fn answer() -> i32 { 42 }\n".repeat(10);
        let compressed = dispatch(
            "aphrodite_compress",
            &serde_json::json!({"content": content}).to_string(),
        );
        let hash = compressed["hash"]
            .as_str()
            .expect("hash present")
            .to_string();
        assert!(!hash.is_empty());

        let retrieved = dispatch(
            "aphrodite_retrieve",
            &serde_json::json!({"hash": hash}).to_string(),
        );
        assert_eq!(
            retrieved["found"], true,
            "retrieve must resolve a just-compressed hash"
        );
        assert_eq!(retrieved["content"], content);
    }

    #[test]
    fn test_retrieve_with_query_filters_lines() {
        let content = "alpha line\nbeta error here\ngamma line\n";
        let c = dispatch(
            "aphrodite_compress",
            &serde_json::json!({"content": content}).to_string(),
        );
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
    fn test_retrieve_missing_hash() {
        let r = dispatch(
            "aphrodite_retrieve",
            &serde_json::json!({"hash": "deadbeefdeadbeefdeadbeef"}).to_string(),
        );
        assert_eq!(r["found"], false);
    }

    #[test]
    fn test_catalog_and_search_see_compressions() {
        let _g = crate::test_guard();
        dispatch(
            "aphrodite_compress",
            &serde_json::json!({"content": "needle_xyz in a haystack\n".repeat(5), "type": "log"})
                .to_string(),
        );
        let cat = dispatch("aphrodite_catalog", "{}");
        assert!(cat["total"].as_u64().unwrap() >= 1);
        let found = dispatch(
            "aphrodite_search",
            &serde_json::json!({"query": "log"}).to_string(),
        );
        assert!(found["total"].as_u64().unwrap() >= 1);
    }

    #[test]
    fn test_test_tool_roundtrips() {
        let r = dispatch(
            "aphrodite_test",
            &serde_json::json!({"mode": "full"}).to_string(),
        );
        assert_eq!(r["status"], "ok", "smoke test should pass: {:?}", r);
        assert_eq!(r["passed"], r["total"]);
    }

    #[test]
    fn test_unknown_tool() {
        let r = dispatch("nonexistent", "{}");
        assert!(r["error"].as_str().unwrap().contains("unknown tool"));
    }

    #[test]
    fn test_prefetch_real_file() {
        let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src/tools.rs");
        let r = dispatch(
            "aphrodite_prefetch",
            &serde_json::json!({"paths": [src]}).to_string(),
        );
        assert_eq!(
            r["loaded"], 1,
            "prefetch should load this source file: {:?}",
            r
        );
    }
}
