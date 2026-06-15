//! aphrodite — Reverse proxy with Chat Completions API support.
//!
//! Two modes:
//! - **Cache** (:9797): In-memory CCR, lightweight compression, no tool injection.
//!   Passes most content through, only compresses very large outputs (>8KB).
//! - **Token** (:9798): SQLite CCR, aggressive compression, tool injection,
//!   tool relay for bidirectional Hermes communication.
//!
//! Chat Completions API:
//! - Forwards POST /v1/chat/completions to DeepSeek
//! - Intercepts responses, compresses tool output via CCR
//! - Injects aphrodite_retrieve tool definition into tool_calls when aphrodite mode

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use axum::{
    body::Body,
    extract::State,
    http::{Method, StatusCode},
    response::{IntoResponse, Json, Response},
};
use bytes::Bytes;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

use headroom_core::ccr::backends::in_memory::InMemoryCcrStore;
use headroom_core::ccr::backends::sqlite::SqliteCcrStore;
use headroom_core::ccr::{compute_key, CcrStore};

/// API key wrapper with safe Debug — never leaks to logs.
#[derive(Clone)]
pub struct Secret(pub(crate) String);

impl std::fmt::Debug for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[REDACTED]")
    }
}

impl std::fmt::Display for Secret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for Secret {
    fn from(s: &str) -> Self { Secret(s.to_string()) }
}

impl From<String> for Secret {
    fn from(s: String) -> Self { Secret(s) }
}

use crate::config::{Cli, ProxyMode};

// ── Constants ───────────────────────────────────────────────────────

/// Content size threshold for cache mode compression (8KB).
const CACHE_COMPRESS_THRESHOLD: usize = 8192;
/// Content size threshold for aphrodite mode compression (1KB).
const TOKEN_COMPRESS_THRESHOLD: usize = 1024;
/// Chat Completions API path.
const CHAT_COMPLETIONS_PATH: &str = "/v1/chat/completions";

// ── State ──────────────────────────────────────────────────────────

pub struct AppState {
    pub client: HttpClient,
    pub api_url: String,
    pub model: String,
    pub api_key: Secret,
    pub ccr: Option<Arc<dyn CcrStore>>,
    pub add_markers: bool,
    pub mode: ProxyMode,
    pub tool_relay: bool,
    pub notify_url: Option<String>,
    pub notify_key: Option<String>,
    /// Dev mode — verbose logging.
    pub dev: bool,

    // Structured debug
    /// Ring buffer of last 50 request summaries
    pub request_history: std::sync::Mutex<Vec<serde_json::Value>>,
    /// Inline CCR for tiny entries — no round-trip needed (< INLINE_CCR_THRESHOLD bytes)
    pub inline_ccr: std::sync::Mutex<std::collections::HashMap<String, String>>,

    // Stats
    /// Latency histogram buckets (microseconds): 1ms, 10ms, 100ms, 1s, 10s
    pub latency_buckets: [AtomicU64; 5],
    /// Track last N errors for hot-path analysis
    pub last_errors: std::sync::Mutex<Vec<String>>,
    /// Compression decision counters by content type
    pub compressions_by_type: std::sync::Mutex<std::collections::HashMap<String, u64>>,

    // Stats
    pub requests_total: AtomicU64,
    pub requests_compressed: AtomicU64,
    pub tokens_saved: AtomicU64,
    pub ccr_hits: AtomicU64,
    pub ccr_misses: AtomicU64,
    pub ccr_created: AtomicU64,
    pub tool_relay_calls: AtomicU64,
    pub compression_ratio_ema: AtomicU64,  // ×100 for EMA of compression ratio
}

impl AppState {
    pub fn stats_json(&self) -> serde_json::Value {
        serde_json::json!({
            "mode": match self.mode {
                ProxyMode::Cache => "cache",
                ProxyMode::Token => "token",
            },
            "proxy": "aphrodite",
            "ccr_backend": if self.ccr.is_some() { "enabled" } else { "none" },
            "tool_relay": self.tool_relay,
            "requests": {
                "total": self.requests_total.load(Ordering::Relaxed),
                "compressed": self.requests_compressed.load(Ordering::Relaxed),
            },
            "tokens_saved": self.tokens_saved.load(Ordering::Relaxed),
            "ccr": {
                "hits": self.ccr_hits.load(Ordering::Relaxed),
                "misses": self.ccr_misses.load(Ordering::Relaxed),
                "created": self.ccr_created.load(Ordering::Relaxed),
            },
            "tool_relay_calls": self.tool_relay_calls.load(Ordering::Relaxed),
            "latency_buckets_us": [
                self.latency_buckets[0].load(Ordering::Relaxed),
                self.latency_buckets[1].load(Ordering::Relaxed),
                self.latency_buckets[2].load(Ordering::Relaxed),
                self.latency_buckets[3].load(Ordering::Relaxed),
                self.latency_buckets[4].load(Ordering::Relaxed),
            ],
            "compressions_by_type": self.compressions_by_type.lock().map(|m| m.clone()).unwrap_or_default(),
            "compression_ratio_ema": self.compression_ratio_ema.load(Ordering::Relaxed) as f64 / 100.0,
            "last_errors": self.last_errors.lock().map(|v| v.iter().rev().take(5).cloned().collect::<Vec<_>>()).unwrap_or_default(),
            "request_history": self.request_history.lock().map(|v| v.clone()).unwrap_or_default(),
        })
    }

    fn compress_threshold(&self) -> usize {
        match self.mode {
            ProxyMode::Cache => CACHE_COMPRESS_THRESHOLD,
            ProxyMode::Token => TOKEN_COMPRESS_THRESHOLD,
        }
    }

    /// Per-type threshold — code stays in context longer, logs compressed aggressively.
    fn threshold_for(&self, ct: &str) -> usize {
        let base = self.compress_threshold();
        // Auto-tune: adjust thresholds based on historical compression ratios
        let ratio = self.compression_ratio_ema.load(Ordering::Relaxed) as f64 / 100.0;
        let tune = if ratio > 20.0 {
            // Very aggressive — raise thresholds to preserve more content
            2.0
        } else if ratio < 3.0 && ratio > 0.0 {
            // Very conservative — lower thresholds to compress more
            0.5
        } else {
            1.0
        };
        let base = (base as f64 * tune) as usize;
        match ct {
            "error" => base * 8,
            "code_rust" | "code_python" | "code_go" | "code_js" | "code" => base * 4,
            "diff" | "git" => base * 2,
            "tool_output" => base,
            "build_output" | "log" => base / 2,
            "json" => base,
            _ => base,
        }
    }

    fn update_compression_ratio(&self, original_len: usize, compressed_len: usize) {
        if original_len == 0 || compressed_len == 0 { return; }
        let ratio = (original_len as f64 / compressed_len as f64 * 100.0) as u64;
        // Exponential moving average: new = 0.2 * ratio + 0.8 * old
        let old = self.compression_ratio_ema.load(Ordering::Relaxed);
        let new = ((ratio as f64 * 0.2) + (old as f64 * 0.8)) as u64;
        self.compression_ratio_ema.store(new, Ordering::Relaxed);
    }

    fn record_latency(&self, d: std::time::Duration) {
        let us = d.as_micros() as u64;
        let bucket = if us < 1_000 { 0 } else if us < 10_000 { 1 } else if us < 100_000 { 2 } else if us < 1_000_000 { 3 } else { 4 };
        self.latency_buckets[bucket].fetch_add(1, Ordering::Relaxed);
    }

    fn record_error(&self, msg: String) {
        if let Ok(mut v) = self.last_errors.lock() {
            v.push(msg);
            if v.len() > 100 { v.remove(0); }
        }
    }

    fn record_compression(&self, ct: &str) {
        if let Ok(mut m) = self.compressions_by_type.lock() {
            *m.entry(ct.to_string()).or_insert(0) += 1;
        }
    }

    fn record_request(&self, id: &str, method: &str, path: &str, status: u16, compressed: bool, elapsed_ms: u128) {
        if let Ok(mut hist) = self.request_history.lock() {
            hist.push(serde_json::json!({
                "id": id,
                "method": method,
                "path": path,
                "status": status,
                "compressed": compressed,
                "elapsed_ms": elapsed_ms,
            }));
            if hist.len() > 50 { hist.remove(0); }
        }
    }
}

// ── Tool relay types ────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ToolRelayRequest {
    pub tool: String,
    pub params: serde_json::Value,
    pub callback_url: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolRelayResponse {
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub async_call: bool,
}

// ── CCR management types ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CcrCreateRequest {
    pub content: String,
    pub key: Option<String>,
    pub ttl_seconds: Option<u64>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CcrCreateResponse {
    pub hash: String,
    pub compression_ratio: f64,
    pub original_size: usize,
    pub compressed_size: usize,
}

#[derive(Debug, Serialize)]
pub struct CcrNotification {
    pub event: String,
    pub hash: String,
    pub created_at: u64,
    pub ttl: u64,
    pub tags: Vec<String>,
}

// ── Build state ─────────────────────────────────────────────────────

pub async fn build_state(cli: &Cli) -> anyhow::Result<AppState> {
    let client = HttpClient::builder()
        .timeout(std::time::Duration::from_secs(cli.timeout))
        .build()?;

    let ccr: Option<Arc<dyn CcrStore>> = match cli.mode {
        ProxyMode::Token if !cli.no_ccr_marker => {
            let store = SqliteCcrStore::open(&cli.ccr_db_path, cli.ccr_ttl_seconds)
                .map_err(|e| anyhow::anyhow!("SQLite CCR: {}", e))?;
            Some(Arc::new(store))
        }
        ProxyMode::Cache => {
            let store = InMemoryCcrStore::with_capacity_and_ttl(
                10_000,
                std::time::Duration::from_secs(cli.ccr_ttl_seconds),
            );
            Some(Arc::new(store))
        }
        _ => None,
    };

    Ok(AppState {
        client,
        api_url: cli.api_url.clone(),
        model: cli.model.clone(),
        api_key: cli.api_key.clone().into(),
        ccr,
        add_markers: !cli.no_ccr_marker,
        mode: cli.mode,
        tool_relay: cli.tool_relay,
        notify_url: cli.notify_url.clone(),
        notify_key: cli.notify_key.clone(),
        dev: cli.dev,
        latency_buckets: [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)],
        last_errors: Mutex::new(Vec::new()),
        compressions_by_type: Mutex::new(HashMap::new()),
        request_history: Mutex::new(Vec::new()),
        inline_ccr: Mutex::new(std::collections::HashMap::new()),
        requests_total: AtomicU64::new(0),
        requests_compressed: AtomicU64::new(0),
        tokens_saved: AtomicU64::new(0),
        ccr_hits: AtomicU64::new(0),
        ccr_misses: AtomicU64::new(0),
        ccr_created: AtomicU64::new(0),
        tool_relay_calls: AtomicU64::new(0),
        compression_ratio_ema: AtomicU64::new(10000),  // initial: 100.0x ratio = neutral
    })
}

// ── Main proxy handler ──────────────────────────────────────────────



/// Generate a simple summary — first 3 lines or first 200 chars.
#[allow(dead_code)]
fn generate_summary(content: &str) -> String {
    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).take(3).collect();
    if lines.len() >= 2 {
        format!("[summary] {} lines, {}B: {}", content.lines().count(), content.len(), lines.join(" | "))
    } else {
        let preview = &content[..content.len().min(200)];
        format!("[summary] {}B: {}", content.len(), preview)
    }
}

/// Catch-all proxy handler — forwards any request to DeepSeek.
/// Specifically handles Chat Completions API at /v1/chat/completions.
pub async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    method: Method,
    path: axum::extract::OriginalUri,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    state.requests_total.fetch_add(1, Ordering::Relaxed);
    let t0 = std::time::Instant::now();
    let req_id = uuid::Uuid::new_v4().to_string();
    let req_id_short = &req_id[..8];

    if state.dev {
        // Log incoming headers
        let mut hdr_log = String::new();
        for (k, v) in headers.iter() {
            let val = v.to_str().unwrap_or("?");
            if k.as_str().to_lowercase() != "authorization" {
                hdr_log.push_str(&format!("  {}: {}", k.as_str(), if val.len() > 80 { &val[..80] } else { val }));
            } else {
                hdr_log.push_str("  authorization: [REDACTED]");
            }
            hdr_log.push('\n');
        }
        tracing::info!(
            id = %req_id_short,
            method = %method,
            path = %path.path(),
            body_len = body.len(),
            headers = %hdr_log,
            ">>> REQ"
        );
    }

    let deepseek_path = path.path().trim_start_matches('/');
    let url = format!("{}/{}", state.api_url.trim_end_matches('/'), deepseek_path);

    let is_chat_completion = deepseek_path == CHAT_COMPLETIONS_PATH.trim_start_matches('/');

    let body_vec = body.to_vec();
    let mut upstream_result = Err("unreachable".to_string());
    for attempt in 1..=3u32 {
        let req = state.client.request(method.clone(), &url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", state.api_key));
        let mut req = req;
        for (key, val) in headers.iter() {
            let k = key.as_str().to_lowercase();
            if k != "host" && k != "authorization" && k != "content-length" {
                req = req.header(key, val);
            }
        }
        match req.body(body_vec.clone()).send().await {
            Ok(r) => { upstream_result = Ok(r); break; }
            Err(e) => {
                if attempt < 3 {
                    let ms = 100 * 2u64.pow(attempt - 1);
                    tracing::warn!(attempt, backoff_ms = ms, "upstream retry after error: {}", e);
                    tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
                } else {
                    upstream_result = Err(format!("{}", e));
                }
            }
        }
    }
    match upstream_result {
        Ok(response) => {
            let status = response.status();
            // Extract content-type before consuming response body
            let content_type = response.headers().get("content-type").cloned();
            let resp_body = response.bytes().await.unwrap_or_default();

            // Only compress Chat Completions responses
            if is_chat_completion && state.ccr.is_some() {
                if let Some(compressed) = compress_chat_completion(
                    &state, &resp_body,
                ).await {
                    state.requests_compressed.fetch_add(1, Ordering::Relaxed);
                    state.record_latency(t0.elapsed());
                    state.record_request(req_id_short, method.as_str(), path.path(), status.as_u16(), true, t0.elapsed().as_millis());
                    if state.dev {
                        let elapsed = t0.elapsed();
                        let comp_len = serde_json::to_vec(&compressed).map(|v| v.len()).unwrap_or(0);
                        tracing::info!(
                            id = %req_id_short,
                            status = %status,
                            original_len = resp_body.len(),
                            compressed_len = comp_len,
                            ratio = format!("{:.1}x", resp_body.len() as f64 / comp_len.max(1) as f64),
                            elapsed_ms = elapsed.as_millis(),
                            "<<< COMPRESSED"
                        );
                    }
                    let body = serde_json::to_vec(&compressed).unwrap_or_else(|_| resp_body.to_vec());
                    return Response::builder()
                        .status(status)
                        .header("Content-Type", "application/json")
                        .header("X-Aphrodite-Compressed", "true")
                        .body(Body::from(body))
                        .unwrap();
                }
            }

            if state.dev {
                let elapsed = t0.elapsed();
                let body_preview = if resp_body.len() > 500 {
                    format!("{}... ({} total)", std::str::from_utf8(&resp_body[..200]).unwrap_or("?"), resp_body.len())
                } else {
                    std::str::from_utf8(&resp_body).unwrap_or("?").to_string()
                };
                tracing::info!(
                    id = %req_id_short,
                    status = %status,
                    resp_len = resp_body.len(),
                    elapsed_ms = elapsed.as_millis(),
                    body = %body_preview,
                    "<<< RES"
                );
            }
            // Use already-extracted content_type (fetched before response.bytes())
            state.record_latency(t0.elapsed());
            state.record_request(req_id_short, method.as_str(), path.path(), status.as_u16(), false, t0.elapsed().as_millis());
            let mut builder = Response::builder().status(status);
            if let Some(ct) = content_type {
                builder = builder.header("Content-Type", ct);
            }
            builder.body(Body::from(resp_body)).unwrap()
        }
        Err(e) => {
            state.record_latency(t0.elapsed());
            state.record_request(req_id_short, method.as_str(), path.path(), 502, false, t0.elapsed().as_millis());
            state.record_error(format!("upstream: {}", e));
            if state.dev {
                tracing::error!(
                    id = %req_id_short,
                    error = %e,
                    elapsed_ms = t0.elapsed().as_millis(),
                    "<<< ERR"
                );
            }
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("upstream: {}", e)})),
            ).into_response()
        }
    }
}

/// Detect content type for adaptive compression strategy.

fn detect_content_type(content: &str) -> &'static str {
    let first_line = content.lines().next().unwrap_or("");
    
    // Structured output detection
    if content.starts_with('{') || content.starts_with('[') {
        if content.contains("exit_code") || content.contains("\"status\"") {
            return "tool_output";
        }
        return "json";
    }
    
    // Error output — always keep visible
    if first_line.contains("error") || first_line.contains("Error") || first_line.contains("ERROR")
        || first_line.contains("Traceback") || first_line.contains("panic")
        || first_line.starts_with("thread '") 
    {
        return "error";
    }
    
    // Build/test output patterns
    if first_line.starts_with("Compiling ") || first_line.starts_with("   Compiling ")
        || first_line.contains("Finished") || first_line.starts_with("running ")
        || first_line.starts_with("test ") 
    {
        return "build_output";
    }
    
    // Linter output patterns
    if first_line.starts_with("error[E") || first_line.starts_with("error: ")
        || first_line.starts_with("warning[") || first_line.starts_with("warning: ")
        || first_line.contains("|") && (first_line.contains("error") || first_line.contains("warning"))
        || first_line.contains("mypy") || first_line.contains("clippy") 
        || first_line.contains("eslint") || first_line.contains("tsc ")
    {
        return "linter";
    }
    
    // Diff output
    if first_line.starts_with("diff --git ") || first_line.starts_with("@@ -")
        || first_line.starts_with("+++ ") || first_line.starts_with("--- ")
    {
        return "diff";
    }
    
    // Git output
    if first_line.starts_with("commit ") || first_line.starts_with("On branch ")
    {
        return "git";
    }
    
    // Code detection — language-specific
    if content.lines().count() > 3 {
        // Rust
        if content.contains("fn ") && (content.contains("-> ") || content.contains("impl ") 
            || content.contains("struct ") || content.contains("pub ")) 
        {
            return "code_rust";
        }
        // Python
        if content.contains("def ") && (content.contains("import ") || content.contains("class ")
            || content.contains("from ") || content.contains("self."))
        {
            return "code_python";
        }
        // Go
        if (content.contains("func ") || content.contains("package ")) 
            && content.contains("import (") 
        {
            return "code_go";
        }
        // JS/TS
        if (content.contains("function ") || content.contains("const ") || content.contains("=> "))
            && (content.contains("import ") || content.contains("export "))
        {
            return "code_js";
        }
        // Generic code
        if content.contains("fn ") || content.contains("def ") || content.contains("class ")
            || content.contains("import ") || content.contains("pub fn")
        {
            return "code";
        }
    }
    
    // Terminal output
    if content.lines().count() > 5 {
        return "log";
    }
    "text"
}

/// Create a standard CCR marker the LLM can parse to decide retrieval.
fn smart_marker(hash: &str, content: &str, ct: &str) -> String {
    let size = content.len();
    let preview = &content[..content.len().min(120)];
    let oneliner = preview.lines().next().unwrap_or(preview).trim();
    format!("<<<CCR:{}|{}|{}>>> {}", hash, ct, size, oneliner)
}

/// Compress a Chat Completions API response with smart markers.
async fn compress_chat_completion(
    state: &AppState,
    resp_body: &[u8],
) -> Option<serde_json::Value> {
    let mut response: serde_json::Value = serde_json::from_slice(resp_body).ok()?;
    let choices = response.get_mut("choices")?.as_array_mut()?;
    let base_threshold = state.compress_threshold();  // floor threshold for all types
    let mut did_compress = false;

    for choice in choices {
        let message = choice.get_mut("message")?;

        // Compress text content with smart markers
        if let Some(content_val) = message.get_mut("content") {
            if let Some(content) = content_val.as_str() {
                let ct = detect_content_type(content);
                let threshold = state.threshold_for(ct).max(base_threshold);
                if content.len() > threshold {
                    if let Some(ccr) = &state.ccr {
                        let hash = compute_key(content.as_bytes());
                        if ccr.get(&hash).is_some() {
                            state.ccr_hits.fetch_add(1, Ordering::Relaxed);
                        } else {
                            state.ccr_misses.fetch_add(1, Ordering::Relaxed);
                            ccr.put(&hash, content);
                            state.ccr_created.fetch_add(1, Ordering::Relaxed);
                            state.tokens_saved.fetch_add((content.len() - hash.len()) as u64, Ordering::Relaxed);
                        }

                        let (compressed, orig_len) = {
                            let ct = detect_content_type(content);
                            let compressed = match state.mode {
                                ProxyMode::Cache => {
                                    let preview = &content[..content.len().min(512)];
                                    format!("<<<CCR:{}|{}|{}>>>\n{}", hash, ct, content.len(), preview)
                                }
                                ProxyMode::Token => {
                                    smart_marker(&hash, content, ct)
                                }
                            };
                            let len = content.len();
                            state.record_compression(ct);
                            (compressed, len)
                        };
                        *content_val = serde_json::Value::String(compressed);
                        did_compress = true;
                        state.update_compression_ratio(orig_len, hash.len());
                    }
                }
            }
        }

        // Compress tool call outputs
        if let Some(tool_calls_val) = message.get_mut("tool_calls") {
            if let Some(arr) = tool_calls_val.as_array_mut() {
                for tc in arr.iter_mut() {
                    if let Some(func) = tc.get_mut("function") {
                        if let Some(args) = func.get_mut("arguments") {
                            if let Some(args_str) = args.as_str() {
                                let args_owned = args_str.to_string();  // drop borrow before mutation
                                let ct = detect_content_type(&args_owned);
                                let threshold = state.threshold_for(ct).max(base_threshold);
                                if args_owned.len() > threshold {
                                    if let Some(ccr) = &state.ccr {
                                        let hash = compute_key(args_owned.as_bytes());
                                        if ccr.get(&hash).is_some() {
                                            state.ccr_hits.fetch_add(1, Ordering::Relaxed);
                                        } else {
                                            state.ccr_misses.fetch_add(1, Ordering::Relaxed);
                                            ccr.put(&hash, &args_owned);
                                            state.ccr_created.fetch_add(1, Ordering::Relaxed);
                                            state.tokens_saved.fetch_add((args_owned.len() - hash.len()) as u64, Ordering::Relaxed);
                                        }
                                        let (compressed, orig_len) = {
                                            let ct2 = detect_content_type(&args_owned);
                                            let compressed = smart_marker(&hash, &args_owned, ct2);
                                            let len = args_owned.len();
                                            state.record_compression(ct2);
                                            (compressed, len)
                                        };
                                        *args = serde_json::Value::String(compressed);
                                        did_compress = true;
                        state.update_compression_ratio(orig_len, hash.len());
                                    }
                                }
                            }
                        }
                    }
                }

                // Tool injection removed — aphrodite_retrieve is registered by the Python plugin.
                // Injecting into the response tool_calls array was incorrect (Bug 18).
            }
        }
    }

    if did_compress { Some(response) } else { None }
}

// ── Tool relay handler ───────────────────────────────────────────────

pub async fn handle_tool_relay(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ToolRelayRequest>,
) -> impl IntoResponse {
    state.tool_relay_calls.fetch_add(1, Ordering::Relaxed);
    tracing::info!(tool = %req.tool, "tool_relay");

    if let Some(cb) = &req.callback_url {
        let state = state.clone();
        let tool = req.tool.clone();
        let params = req.params.clone();
        let cb = cb.clone();
        tokio::spawn(async move {
            let result = execute_tool_relay(&state, &tool, &params).await;
            let _ = state.client.post(&cb).json(&result).send().await;
        });
        return Json(ToolRelayResponse { success: true, result: None, error: None, async_call: true });
    }

    match execute_tool_relay(&state, &req.tool, &req.params).await {
        Ok(val) => Json(ToolRelayResponse { success: true, result: Some(val), error: None, async_call: false }),
        Err(e) => Json(ToolRelayResponse { success: false, result: None, error: Some(e), async_call: false }),
    }
}

async fn execute_tool_relay(
    state: &AppState, tool: &str, params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    match tool {
        "aphrodite_retrieve" => {
            let hash = params.get("hash").and_then(|v| v.as_str()).ok_or("missing hash")?;
            if let Some(ccr) = &state.ccr {
                match ccr.get(hash) {
                    Some(content) => Ok(serde_json::json!({"found": true, "content": content})),
                    None => Ok(serde_json::json!({"found": false})),
                }
            } else {
                Err("CCR not enabled".into())
            }
        }
        "aphrodite_compress" => {
            let content = params.get("content").and_then(|v| v.as_str()).ok_or("missing content")?;
            if let Some(ccr) = &state.ccr {
                let hash = compute_key(content.as_bytes());
                ccr.put(&hash, content);
                Ok(serde_json::json!({"compressed": format!("<<<CCR:{}|compress|0>>>", hash), "hash": hash}))
            } else {
                Err("CCR not enabled".into())
            }
        }
        "aphrodite_list" => {
            match &state.ccr {
                Some(ccr) => Ok(serde_json::json!({
                    "entries": ccr.len(),
                    "backend": match state.mode {
                        ProxyMode::Cache => "in_memory",
                        ProxyMode::Token => "sqlite",
                    },
                })),
                None => Ok(serde_json::json!({"entries": 0, "message": "CCR not enabled"})),
            }
        }
        _ => Err(format!("Unknown tool: {}", tool)),
    }
}

// ── Programmatic CCR handlers ────────────────────────────────────────

pub async fn handle_ccr_create(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CcrCreateRequest>,
) -> impl IntoResponse {
    let original_size = req.content.len();
    let hash = req.key.unwrap_or_else(|| compute_key(req.content.as_bytes()));

    if let Some(ccr) = &state.ccr {
        ccr.put(&hash, &req.content);
        state.ccr_created.fetch_add(1, Ordering::Relaxed);
        state.tokens_saved.fetch_add(original_size.saturating_sub(hash.len()) as u64, Ordering::Relaxed);

        // Background summary disabled — burns process + extra CCR entries
        // if req.content.len() > 1024 { ... }
    }

    if let Some(notify_url) = &state.notify_url {
        let notification = CcrNotification {
            event: "ccr_created".into(),
            hash: hash.clone(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            ttl: req.ttl_seconds.unwrap_or(3600),
            tags: req.tags.unwrap_or_default(),
        };
        let client = state.client.clone();
        let url = notify_url.clone();
        tokio::spawn(async move { let _ = client.post(&url).json(&notification).send().await; });
    }

    let compressed_size = hash.len();
    Json(CcrCreateResponse {
        hash,
        compression_ratio: if original_size > 0 { original_size as f64 / compressed_size.max(1) as f64 } else { 1.0 },
        original_size,
        compressed_size,
    })
}

pub async fn handle_ccr_list(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    match &state.ccr {
        Some(ccr) => Json(serde_json::json!({
            "entries": ccr.len(),
            "backend": match state.mode {
                ProxyMode::Cache => "in_memory",
                ProxyMode::Token => "sqlite",
            },
            "mode": match state.mode {
                ProxyMode::Cache => "cache",
                ProxyMode::Token => "token",
            },
        })),
        None => Json(serde_json::json!({"entries": 0, "message": "CCR not enabled"})),
    }
}



// ── Health check ────────────────────────────────────────────────────

pub async fn health_check(
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // Local-only health — no upstream API call (done separately via /health/upstream)
    // Always 200 — capability state conveyed via JSON body (CCR is optional/opt-in)
    let ccr_ok = state.ccr.is_some();

    (StatusCode::OK, Json(serde_json::json!({
        "status": if ccr_ok { "healthy" } else { "degraded" },
        "ccr": ccr_ok,
        "mode": match state.mode {
            ProxyMode::Cache => "cache",
            ProxyMode::Token => "token",
        },
        "version": env!("CARGO_PKG_VERSION"),
    }))).into_response()
}

// ── Tests ────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_threshold_cache() {
        use std::sync::Mutex;
        use std::collections::HashMap;
        let state = AppState {
            client: HttpClient::new(),
            api_url: "https://upstream-openai.com".into(),
            model: "test".into(),
            api_key: "test".into(),
            ccr: None,
            add_markers: false,
            mode: ProxyMode::Cache,
            tool_relay: false,
            notify_url: None,
            notify_key: None,
            dev: false,
            requests_total: AtomicU64::new(0),
            requests_compressed: AtomicU64::new(0),
            tokens_saved: AtomicU64::new(0),
            ccr_hits: AtomicU64::new(0),
            ccr_misses: AtomicU64::new(0),
            ccr_created: AtomicU64::new(0),
            tool_relay_calls: AtomicU64::new(0),
        compression_ratio_ema: AtomicU64::new(10000),  // initial: 100.0x ratio = neutral
            request_history: Mutex::new(Vec::new()),
        inline_ccr: Mutex::new(std::collections::HashMap::new()),
            latency_buckets: [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)],
            last_errors: Mutex::new(Vec::new()),
            compressions_by_type: Mutex::new(HashMap::new()),
        };
        assert_eq!(state.compress_threshold(), CACHE_COMPRESS_THRESHOLD);
    }

    #[test]
    fn test_compress_threshold_aphrodite() {
        let state = AppState {
            mode: ProxyMode::Token,
            ..test_state()
        };
        assert_eq!(state.compress_threshold(), TOKEN_COMPRESS_THRESHOLD);
    }

    #[test]
    fn test_stats_json_modes() {
        let cache = test_state();
        let stats = cache.stats_json();
        assert_eq!(stats["mode"], "cache");
        assert_eq!(stats["proxy"], "aphrodite");

        let mut aph = test_state();
        aph.mode = ProxyMode::Token;
        let stats = aph.stats_json();
        assert_eq!(stats["mode"], "token");
    }

    #[test]
    fn test_ccr_create_response() {
        let resp = CcrCreateResponse {
            hash: "abc123".into(),
            compression_ratio: 2.5,
            original_size: 100,
            compressed_size: 40,
        };
        assert_eq!(resp.hash, "abc123");
        assert!((resp.compression_ratio - 2.5).abs() < 0.01);
    }

    #[test]
    fn test_tool_relay_response_sync() {
        let resp = ToolRelayResponse {
            success: true,
            result: Some(serde_json::json!({"found": true})),
            error: None,
            async_call: false,
        };
        assert!(resp.success);
        assert!(!resp.async_call);
    }

    #[test]
    fn test_tool_relay_response_async() {
        let resp = ToolRelayResponse {
            success: true,
            result: None,
            error: None,
            async_call: true,
        };
        assert!(resp.async_call);
    }

    fn test_state() -> AppState {
        use std::sync::Mutex;
        use std::collections::HashMap;
        AppState {
            client: HttpClient::new(),
            api_url: "https://upstream-openai.com".into(),
            model: "default-model".into(),
            api_key: "test".into(),
            ccr: None,
            add_markers: false,
            mode: ProxyMode::Cache,
            tool_relay: false,
            notify_url: None,
            notify_key: None,
            dev: false,
            requests_total: AtomicU64::new(0),
            requests_compressed: AtomicU64::new(0),
            tokens_saved: AtomicU64::new(0),
            ccr_hits: AtomicU64::new(0),
            ccr_misses: AtomicU64::new(0),
            ccr_created: AtomicU64::new(0),
            tool_relay_calls: AtomicU64::new(0),
        compression_ratio_ema: AtomicU64::new(10000),  // initial: 100.0x ratio = neutral
            request_history: Mutex::new(Vec::new()),
        inline_ccr: Mutex::new(std::collections::HashMap::new()),
            latency_buckets: [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)],
            last_errors: Mutex::new(Vec::new()),
            compressions_by_type: Mutex::new(HashMap::new()),
        }
    }
}
