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
//! - Injects headroom_retrieve tool definition into tool_calls when aphrodite mode

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
use headroom_core::ccr::{compute_key, marker_for, CcrStore};

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
    pub api_key: String,
    pub ccr: Option<Arc<dyn CcrStore>>,
    pub inject_tool: bool,
    pub add_markers: bool,
    pub mode: ProxyMode,
    pub tool_relay: bool,
    pub notify_url: Option<String>,
    pub notify_key: Option<String>,
    /// Dev mode — verbose logging.
    pub dev: bool,

    // Structured debug
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
            "last_errors": self.last_errors.lock().map(|v| v.iter().rev().take(5).cloned().collect::<Vec<_>>()).unwrap_or_default(),
        })
    }

    fn compress_threshold(&self) -> usize {
        match self.mode {
            ProxyMode::Cache => CACHE_COMPRESS_THRESHOLD,
            ProxyMode::Token => TOKEN_COMPRESS_THRESHOLD,
        }
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
        api_key: cli.api_key.clone(),
        ccr,
        inject_tool: !cli.no_ccr_inject_tool && matches!(cli.mode, ProxyMode::Token),
        add_markers: !cli.no_ccr_marker,
        mode: cli.mode,
        tool_relay: cli.tool_relay,
        notify_url: cli.notify_url.clone(),
        notify_key: cli.notify_key.clone(),
        dev: cli.dev,
        latency_buckets: [AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0), AtomicU64::new(0)],
        last_errors: Mutex::new(Vec::new()),
        compressions_by_type: Mutex::new(HashMap::new()),
        requests_total: AtomicU64::new(0),
        requests_compressed: AtomicU64::new(0),
        tokens_saved: AtomicU64::new(0),
        ccr_hits: AtomicU64::new(0),
        ccr_misses: AtomicU64::new(0),
        ccr_created: AtomicU64::new(0),
        tool_relay_calls: AtomicU64::new(0),
    })
}

// ── Main proxy handler ──────────────────────────────────────────────

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
    let _t0 = std::time::Instant::now();

    if state.dev {
        tracing::info!(
            method = %method,
            path = %path.path(),
            body_len = body.len(),
            "req:start"
        );
    }

    let deepseek_path = path.path().trim_start_matches('/');
    let url = format!("{}/{}", state.api_url.trim_end_matches('/'), deepseek_path);

    let is_chat_completion = deepseek_path == CHAT_COMPLETIONS_PATH.trim_start_matches('/');

    let mut req = state
        .client
        .request(method.clone(), &url)
        .header("Authorization", format!("Bearer {}", state.api_key))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json");

    // Forward select headers
    for (key, val) in headers.iter() {
        let k = key.as_str().to_lowercase();
        if k != "host" && k != "authorization" && k != "content-length" {
            req = req.header(key, val);
        }
    }

    let body_vec = body.to_vec();
    match req.body(body_vec.clone()).send().await {
        Ok(response) => {
            let status = response.status();
            let resp_body = response.bytes().await.unwrap_or_default();

            // Only compress Chat Completions responses
            if is_chat_completion && state.ccr.is_some() {
                if let Some(compressed) = compress_chat_completion(
                    &state, &resp_body,
                ).await {
                    state.requests_compressed.fetch_add(1, Ordering::Relaxed);
                    if state.dev {
                        tracing::info!(
                            status = %status,
                            resp_len = resp_body.len(),
                            compressed_len = serde_json::to_vec(&compressed).map(|v| v.len()).unwrap_or(0),
                            "aphrodite dev: compressed response"
                        );
                    }
                    let body = serde_json::to_vec(&compressed).unwrap_or_else(|_| resp_body.to_vec());
                    return Response::builder()
                        .status(status)
                        .header("Content-Type", "application/json")
                        .body(Body::from(body))
                        .unwrap();
                }
            }

            Response::builder()
                .status(status)
                .body(Body::from(resp_body))
                .unwrap()
        }
        Err(e) => {
            state.record_error(format!("upstream: {}", e));
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({"error": format!("upstream: {}", e)})),
            ).into_response()
        }
    }
}

/// Detect content type for adaptive compression strategy.

fn detect_content_type(content: &str) -> &'static str {
    if content.starts_with('{') || content.starts_with('[') {
        if content.contains("exit_code") {
            return "tool_output";
        }
        return "json";
    }
    if content.contains("error") || content.contains("Error") || content.contains("ERROR") {
        return "error";
    }
    if content.contains('\n') && content.lines().count() > 5 {
        if content.contains("fn ") || content.contains("def ") || content.contains("class ") {
            return "code";
        }
        return "log";
    }
    "text"
}

/// Create a smart CCR marker with metadata the LLM can use to decide retrieval.
fn smart_marker(hash: &str, content: &str, ct: &str) -> String {
    let size = content.len();
    let preview = &content[..content.len().min(120)];
    let oneliner = preview.lines().next().unwrap_or(preview).trim();
    format!("⫷CCR:{}|{}|{}⫸ {}", hash, ct, size, oneliner)
}

/// Compress a Chat Completions API response with smart markers.
async fn compress_chat_completion(
    state: &AppState,
    resp_body: &[u8],
) -> Option<serde_json::Value> {
    let mut response: serde_json::Value = serde_json::from_slice(resp_body).ok()?;
    let choices = response.get_mut("choices")?.as_array_mut()?;
    let threshold = state.compress_threshold();
    let mut did_compress = false;

    for choice in choices {
        let message = choice.get_mut("message")?;

        // Compress text content with smart markers
        if let Some(content_val) = message.get_mut("content") {
            if let Some(content) = content_val.as_str() {
                if content.len() > threshold {
                    if let Some(ccr) = &state.ccr {
                        let hash = compute_key(content.as_bytes());
                        ccr.put(&hash, content);
                        let ct = detect_content_type(content);

                        let compressed = match state.mode {
                            ProxyMode::Cache => {
                                // Keep first 512 chars + marker
                                let preview = &content[..content.len().min(512)];
                                format!("⫷CCR:{}|{}|{}⫸\n{}", hash, ct, content.len(), preview)
                            }
                            ProxyMode::Token => {
                                // Marker with one-line preview so LLM knows whether to retrieve
                                smart_marker(&hash, content, ct)
                            }
                        };
                        *content_val = serde_json::Value::String(compressed);
                        did_compress = true;
                        state.record_compression(ct);
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
                                if args_str.len() > threshold {
                                    if let Some(ccr) = &state.ccr {
                                        let hash = compute_key(args_str.as_bytes());
                                        ccr.put(&hash, args_str);
                                        let ct = detect_content_type(args_str);
                                        *args = serde_json::Value::String(
                                            smart_marker(&hash, args_str, ct)
                                        );
                                        did_compress = true;
                        state.record_compression(ct);
                                    }
                                }
                            }
                        }
                    }
                }

                // Inject optimized headroom_retrieve tool
                if state.inject_tool {
                    let retrieve_tool = serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": "headroom_retrieve",
                            "description": "Retrieve original content behind a CCR marker. Call this when you see a ⭷CCR:hash marker and need the full content to answer accurately. Provide the hash from the marker. Optionally filter with query.",
                            "parameters": {
                                "type": "object",
                                "properties": {
                                    "hash": {
                                        "type": "string",
                                        "description": "The hash from the CCR marker (e.g. ⭷CCR:abc123|json|2048⭸)"
                                    },
                                    "query": {
                                        "type": "string",
                                        "description": "Filter returned content to lines matching this query (optional)"
                                    }
                                },
                                "required": ["hash"]
                            }
                        }
                    });
                    arr.push(retrieve_tool);
                }
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
        "headroom_retrieve" => {
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
        "headroom_compress" => {
            let content = params.get("content").and_then(|v| v.as_str()).ok_or("missing content")?;
            if let Some(ccr) = &state.ccr {
                let hash = compute_key(content.as_bytes());
                ccr.put(&hash, content);
                Ok(serde_json::json!({"compressed": marker_for(&hash), "hash": hash}))
            } else {
                Err("CCR not enabled".into())
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
    let upstream_ok = state
        .client
        .get(format!("{}/models", state.api_url.trim_end_matches('/')))
        .header("Authorization", format!("Bearer {}", state.api_key))
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map(|r| r.status().is_success())
        .unwrap_or(false);

    let status_code = if upstream_ok { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };

    (status_code, Json(serde_json::json!({
        "status": if upstream_ok { "healthy" } else { "degraded" },
        "upstream": upstream_ok,
        "ccr": state.ccr.is_some(),
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
        let state = AppState {
            client: HttpClient::new(),
            api_url: "https://api.deepseek.com".into(),
            model: "test".into(),
            api_key: "test".into(),
            ccr: None,
            inject_tool: false,
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
        AppState {
            client: HttpClient::new(),
            api_url: "https://api.deepseek.com".into(),
            model: "deepseek-v4-pro".into(),
            api_key: "test".into(),
            ccr: None,
            inject_tool: false,
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
        }
    }
}
