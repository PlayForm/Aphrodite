//! Hermes tool schemas — JSON Schema definitions for all aphrodite tools.
//! These match exactly what the Python plugin's `_tools.py` and `_hooks/__init__.py` export.

use serde_json::json;

/// Return all tool schemas as a JSON array.
pub fn all_schemas() -> Vec<serde_json::Value> {
    vec![
        schema_compress(),
        schema_retrieve(),
        schema_stats(),
        schema_files(),
        schema_diff(),
        schema_search(),
        schema_test(),
        schema_catalog(),
        schema_reclassify(),
        schema_prefetch(),
        schema_prefetch_status(),
        schema_rebuild(),
    ]
}

/// Get a single tool schema by name.
pub fn get_schema(name: &str) -> Option<serde_json::Value> {
    all_schemas().into_iter().find(|s| s["name"] == name)
}

fn schema_compress() -> serde_json::Value {
    json!({
        "name": "aphrodite_compress",
        "description": "Compress content into CCR via aphrodite proxy for later retrieval. Specify type for adaptive compression: code, log, diff, error, json, build_output.",
        "parameters": {
            "type": "object",
            "properties": {
                "content": {"type": "string", "description": "Content to compress and store in CCR"},
                "type": {"type": "string", "description": "Optional: content type hint - code, log, diff, error, json, build_output, text"},
                "_ccr_center": {"type": "string", "description": "Optional: center string that travels with the marker"}
            },
            "required": ["content"]
        }
    })
}

fn schema_retrieve() -> serde_json::Value {
    json!({
        "name": "aphrodite_retrieve",
        "description": "Resolve CCR markers to original content via aphrodite proxy. Optionally filter by query. Supports file path reads.",
        "parameters": {
            "type": "object",
            "properties": {
                "hash": {"type": "string", "description": "CCR marker hash to retrieve"},
                "query": {"type": "string", "description": "Optional: filter content to lines containing this query"},
                "path": {"type": "string", "description": "Optional: file path to read directly (bypasses CCR)"},
                "depth": {"type": "integer", "description": "Optional: compression depth (1=raw, 2=headroom-reduced)"}
            }
        }
    })
}

fn schema_stats() -> serde_json::Value {
    json!({
        "name": "aphrodite_stats",
        "description": "Check aphrodite proxy health, CCR stats, engine compression status.",
        "parameters": {
            "type": "object",
            "properties": {}
        }
    })
}

fn schema_files() -> serde_json::Value {
    json!({
        "name": "aphrodite_files",
        "description": "List all file paths referenced in the current session.",
        "parameters": {
            "type": "object",
            "properties": {}
        }
    })
}

fn schema_diff() -> serde_json::Value {
    json!({
        "name": "aphrodite_diff",
        "description": "Show conversation turn history — what was discussed, compressed, and stored across turns.",
        "parameters": {
            "type": "object",
            "properties": {}
        }
    })
}

fn schema_search() -> serde_json::Value {
    json!({
        "name": "aphrodite_search",
        "description": "Search across CCR entries — find previously compressed content by keyword or type.",
        "parameters": {
            "type": "object",
            "properties": {
                "query": {"type": "string", "description": "Search keyword or phrase"},
                "type": {"type": "string", "description": "Optional: filter by CCR type"}
            },
            "required": ["query"]
        }
    })
}

fn schema_test() -> serde_json::Value {
    json!({
        "name": "aphrodite_test",
        "description": "Run full smoke test suite — compress, retrieve, search, stats, files, diff, proxy health.",
        "parameters": {
            "type": "object",
            "properties": {
                "mode": {"type": "string", "description": "Test mode: quick (default), full, or matrix"}
            }
        }
    })
}

fn schema_catalog() -> serde_json::Value {
    json!({
        "name": "aphrodite_catalog",
        "description": "Return full compression catalog with hashes, sizes, types, previews. Mode 'toc' for compact table-of-contents.",
        "parameters": {
            "type": "object",
            "properties": {
                "mode": {"type": "string", "description": "Optional: 'toc' for compact table-of-contents, default full catalog"}
            }
        }
    })
}

fn schema_reclassify() -> serde_json::Value {
    json!({
        "name": "aphrodite_reclassify",
        "description": "Retroactively classify/metadata-enrich all CCR entries lacking structured metadata.",
        "parameters": {
            "type": "object",
            "properties": {
                "hash": {"type": "string", "description": "Optional: reclassify a single entry by hash"},
                "action": {"type": "string", "description": "Set to 'all' to reclassify all entries lacking meta."}
            }
        }
    })
}

fn schema_prefetch() -> serde_json::Value {
    json!({
        "name": "aphrodite_prefetch",
        "description": "Read files in background and compress to CCR. Returns markers instantly.",
        "parameters": {
            "type": "object",
            "properties": {
                "paths": {"type": "array", "items": {"type": "string"}, "description": "List of file paths to prefetch"}
            },
            "required": ["paths"]
        }
    })
}

fn schema_prefetch_status() -> serde_json::Value {
    json!({
        "name": "aphrodite_prefetch_status",
        "description": "Live prefetch schedule — what's loading, what's ready, ETAs per file.",
        "parameters": {
            "type": "object",
            "properties": {}
        }
    })
}

fn schema_rebuild() -> serde_json::Value {
    json!({
        "name": "aphrodite_rebuild",
        "description": "Rebuild aphrodite crate from source and install binary. Use after code changes.",
        "parameters": {
            "type": "object",
            "properties": {}
        }
    })
}
