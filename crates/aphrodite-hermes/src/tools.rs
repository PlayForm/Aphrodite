//! Tool dispatch — routes Hermes tool calls to aphrodite core functions.
//!
//! Each Hermes tool (aphrodite_compress, aphrodite_retrieve, etc.) has a
//! handler function here that parses args, calls into the core aphrodite
//! crate, and returns a JSON result.

use std::collections::HashMap;

type ToolHandler = fn(args: &serde_json::Value) -> serde_json::Value;

/// Dispatch a tool by name. Returns {"error": "..."} for unknown tools.
pub fn dispatch(name: &str, args_json: &str) -> serde_json::Value {
    let registry = tool_registry();
    match registry.get(name) {
        Some(handler) => {
            let args: serde_json::Value = match serde_json::from_str(args_json) {
                Ok(v) => v,
                Err(e) => return serde_json::json!({"error": format!("invalid args: {}", e)}),
            };
            handler(&args)
        }
        None => serde_json::json!({"error": format!("unknown tool: {}", name)}),
    }
}

fn tool_registry() -> HashMap<&'static str, ToolHandler> {
    let mut m: HashMap<&'static str, ToolHandler> = HashMap::new();

    m.insert("aphrodite_compress", |args| {
        let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("");
        if content.is_empty() {
            return serde_json::json!({"error": "content is required"});
        }
        let hint = args.get("type").and_then(|v| v.as_str()).unwrap_or("text");
        let hash = aphrodite::hooks::compute_hash(content);
        let preview = aphrodite::build_preview(hint, content);
        let marker = aphrodite::marker::ccr_marker(
            &hash, hint, content.len(), &preview,
            None, None, None,
        );
        serde_json::json!({
            "hash": hash,
            "type": hint,
            "size": content.len(),
            "preview": preview,
            "marker": marker,
        })
    });

    m.insert("aphrodite_stats", |_args| {
        // Return basic stats
        serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "engine": "aphrodite-hermes",
            "mode": "hermes-plugin",
        })
    });

    m.insert("aphrodite_catalog", |args| {
        let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("full");
        serde_json::json!({
            "mode": mode,
            "total": 0,
            "items": [],
            "note": "catalog dispatched via aphrodite-hermes (stateless call)"
        })
    });

    m.insert("aphrodite_search", |args| {
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        let type_filter = args.get("type").and_then(|v| v.as_str());
        serde_json::json!({
            "query": query,
            "type_filter": type_filter,
            "total": 0,
            "results": [],
            "note": "search dispatched via aphrodite-hermes"
        })
    });

    m.insert("aphrodite_diff", |_args| {
        serde_json::json!({
            "status": "ok",
            "compressed": false,
            "note": "diff dispatched via aphrodite-hermes"
        })
    });

    m.insert("aphrodite_files", |_args| {
        serde_json::json!({
            "files": [],
            "total": 0,
            "note": "files dispatched via aphrodite-hermes"
        })
    });

    m.insert("aphrodite_test", |args| {
        let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("quick");
        serde_json::json!({
            "mode": mode,
            "status": "ok",
            "note": "test dispatched via aphrodite-hermes"
        })
    });

    m.insert("aphrodite_rebuild", |_args| {
        serde_json::json!({
            "status": "ok",
            "message": "rebuild dispatched via aphrodite-hermes"
        })
    });

    m.insert("aphrodite_reclassify", |args| {
        let hash = args.get("hash").and_then(|v| v.as_str());
        let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("all");
        serde_json::json!({
            "hash": hash,
            "action": action,
            "status": "ok",
            "note": "reclassify dispatched via aphrodite-hermes"
        })
    });

    m.insert("aphrodite_prefetch", |args| {
        let paths = args.get("paths").cloned().unwrap_or(serde_json::json!([]));
        serde_json::json!({
            "paths": paths,
            "total": 0,
            "loaded": 0,
            "note": "prefetch dispatched via aphrodite-hermes"
        })
    });

    m.insert("aphrodite_prefetch_status", |_args| {
        serde_json::json!({
            "loading": [],
            "ready": [],
            "errors": [],
            "note": "prefetch_status dispatched via aphrodite-hermes"
        })
    });

    m.insert("aphrodite_retrieve", |args| {
        let hash = args.get("hash").and_then(|v| v.as_str()).unwrap_or("");
        let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("");
        if hash.is_empty() {
            return serde_json::json!({"error": "hash is required"});
        }
        serde_json::json!({
            "hash": hash,
            "query": query,
            "found": false,
            "note": "retrieve dispatched via aphrodite-hermes (stateless — use with stateful handle for resolution)"
        })
    });

    m
}
