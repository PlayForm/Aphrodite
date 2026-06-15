//! Reverse proxy — forwards to DeepSeek, injects CCR compression.
//!
//! Intercepts chat completions responses, compresses large tool outputs
//! with headroom-core CCR (SQLite-backed), and injects the `headroom_retrieve`
//! tool definition so the LLM can decompress on demand.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use http_body_util::BodyExt;
use serde_json::Value;

use headroom_core::ccr::{self, CcrStore};

use crate::config::Cli;

/// Shared application state.
pub struct AppState {
    /// DeepSeek API base URL
    pub deepseek_url: String,
    /// DeepSeek API key
    pub deepseek_key: String,
    /// Model name
    pub model: String,
    /// HTTP client
    pub client: reqwest::Client,
    /// CCR store (SQLite-backed)
    pub ccr_store: Box<dyn CcrStore>,
    /// Inject CCR retrieval tool into responses
    pub inject_tool: bool,
    /// Add CCR markers to compressed content
    pub add_markers: bool,

    // Stats counters
    pub total_requests: AtomicU64,
    pub compressed_requests: AtomicU64,
    pub total_tokens_saved: AtomicU64,
    pub ccr_stored: AtomicU64,
    pub ccr_hits: AtomicU64,
    pub ccr_misses: AtomicU64,
}

impl AppState {
    pub fn stats_json(&self) -> serde_json::Value {
        serde_json::json!({
            "mode": "token",
            "proxy": "headroom-token",
            "ccr_backend": "sqlite",
            "requests": {
                "total": self.total_requests.load(Ordering::Relaxed),
                "compressed": self.compressed_requests.load(Ordering::Relaxed),
            },
            "tokens_saved": self.total_tokens_saved.load(Ordering::Relaxed),
            "ccr": {
                "stored": self.ccr_stored.load(Ordering::Relaxed),
                "hits": self.ccr_hits.load(Ordering::Relaxed),
                "misses": self.ccr_misses.load(Ordering::Relaxed),
                "entries": self.ccr_store.len(),
            }
        })
    }
}

pub async fn build_state(cli: &Cli) -> anyhow::Result<AppState> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .connect_timeout(std::time::Duration::from_secs(30))
        .build()?;

    let ccr_config = headroom_core::ccr::backends::CcrBackendConfig::Sqlite {
        path: cli.ccr_db_path.clone(),
        ttl_seconds: cli.ccr_ttl_seconds,
    };
    let ccr_store = headroom_core::ccr::backends::from_config(&ccr_config)?;

    tracing::info!(
        db = %cli.ccr_db_path.display(),
        ttl_s = cli.ccr_ttl_seconds,
        "CCR SQLite store initialised"
    );

    Ok(AppState {
        deepseek_url: cli.deepseek_url.trim_end_matches('/').to_string(),
        deepseek_key: cli.deepseek_key.clone(),
        model: cli.model.clone(),
        client,
        ccr_store,
        inject_tool: !cli.no_ccr_inject_tool,
        add_markers: !cli.no_ccr_marker,
        total_requests: AtomicU64::new(0),
        compressed_requests: AtomicU64::new(0),
        total_tokens_saved: AtomicU64::new(0),
        ccr_stored: AtomicU64::new(0),
        ccr_hits: AtomicU64::new(0),
        ccr_misses: AtomicU64::new(0),
    })
}

/// CCR retrieve tool definition — injected into chat completions responses.
const RETRIEVE_TOOL: &str = r#"{
  "type": "function",
  "function": {
    "name": "headroom_retrieve",
    "description": "Resolve CCR markers. Include `path` for local file read. Include `query` for BM25 search.",
    "parameters": {
      "type": "object",
      "properties": {
        "hash": {
          "type": "string",
          "description": "The CCR marker hash (e.g. 'abc123')"
        },
        "path": {
          "type": "string",
          "description": "Optional: local file path to read directly"
        },
        "query": {
          "type": "string",
          "description": "Optional: BM25 search query to filter content"
        }
      },
      "required": ["hash"]
    }
  }
}"#;

/// Main proxy handler — forwards all requests to DeepSeek.
pub async fn proxy_handler(
    State(state): State<Arc<AppState>>,
    req: Request<Body>,
) -> impl IntoResponse {
    state.total_requests.fetch_add(1, Ordering::Relaxed);

    let path = req.uri().path().to_string();
    let method = req.method().clone();
    let upstream_url = format!("{}{}", state.deepseek_url, path);

    let (parts, body) = req.into_parts();
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            tracing::error!(error = %e, "failed to read request body");
            return (StatusCode::BAD_REQUEST, "failed to read body").into_response();
        }
    };

    let mut upstream_req = state
        .client
        .request(method.clone(), &upstream_url)
        .body(body_bytes.clone());

    for (name, value) in &parts.headers {
        let name_str = name.as_str().to_lowercase();
        if name_str == "authorization" {
            upstream_req = upstream_req.header(
                "Authorization",
                format!("Bearer {}", state.deepseek_key),
            );
        } else if name_str != "host" && name_str != "content-length" {
            upstream_req = upstream_req.header(name.as_str(), value.as_bytes());
        }
    }
    upstream_req = upstream_req.header(
        "Authorization",
        format!("Bearer {}", state.deepseek_key),
    );

    let upstream_resp = match upstream_req.send().await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!(error = %e, "upstream request failed");
            return (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": {
                        "message": format!("upstream request failed: {}", e),
                        "type": "proxy_error",
                    }
                })),
            )
                .into_response();
        }
    };

    let status = upstream_resp.status();
    let upstream_headers = upstream_resp.headers().clone();
    let resp_body = match upstream_resp.bytes().await {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "failed to read upstream response");
            return (StatusCode::BAD_GATEWAY, "failed to read upstream body").into_response();
        }
    };

    let is_chat = path.contains("/chat/completions");
    let is_stream = path.contains("stream");

    let final_body = if is_chat && status.is_success() && !is_stream {
        match compress_response(&state, &resp_body) {
            Ok(compressed) => compressed,
            Err(e) => {
                tracing::warn!(error = %e, "compression failed, passing through");
                resp_body.to_vec()
            }
        }
    } else {
        resp_body.to_vec()
    };

    let mut response = Response::builder().status(status);
    for (name, value) in &upstream_headers {
        if let (Ok(n), Ok(v)) = (
            http::HeaderName::from_bytes(name.as_str().as_bytes()),
            http::HeaderValue::from_bytes(value.as_bytes()),
        ) {
            if n.as_str().to_lowercase() != "content-length" {
                response = response.header(n, v);
            }
        }
    }
    response.body(Body::from(final_body)).unwrap().into_response()
}

/// Compress tool output in a chat completions response.
fn compress_response(state: &AppState, body: &[u8]) -> anyhow::Result<Vec<u8>> {
    let body_str = std::str::from_utf8(body)?;
    let mut json: Value = serde_json::from_str(body_str)?;

    let mut did_compress = false;

    if let Some(choices) = json.get_mut("choices").and_then(|c| c.as_array_mut()) {
        for choice in choices {
            if let Some(message) = choice.get_mut("message") {
                if let Some(content) = message.get_mut("content") {
                    if let Some(text) = content.as_str() {
                        if let Some(compressed) = maybe_compress(state, text) {
                            *content = Value::String(compressed);
                            did_compress = true;
                        }
                    } else if let Some(parts) = content.as_array_mut() {
                        for part in parts {
                            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                                if let Some(compressed) = maybe_compress(state, text) {
                                    part["text"] = Value::String(compressed);
                                    did_compress = true;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Inject CCR retrieve tool if we compressed something
    if did_compress && state.inject_tool {
        if let Ok(retrieve_tool) = serde_json::from_str::<Value>(RETRIEVE_TOOL) {
            if let Some(tools) = json.get_mut("tools") {
                if let Some(arr) = tools.as_array_mut() {
                    // Only add if not already present
                    let already_has = arr.iter().any(|t| {
                        t.get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(|n| n.as_str())
                            == Some("headroom_retrieve")
                    });
                    if !already_has {
                        arr.push(retrieve_tool);
                    }
                }
            }
        }
    }

    if did_compress {
        state.compressed_requests.fetch_add(1, Ordering::Relaxed);
    }

    Ok(serde_json::to_vec(&json)?)
}

/// Try to compress a text content block. Returns Some(compressed) if compressed.
fn maybe_compress(state: &AppState, content: &str) -> Option<String> {
    const MIN_CHARS: usize = 400;
    const CHUNK_SIZE: usize = 1500;

    if content.len() < MIN_CHARS {
        return None;
    }

    let token_estimate = content.len() / 3; // rough: ~3 chars per token
    let mut result = String::with_capacity(content.len());
    let mut offset = 0;
    let mut saved = 0usize;

    while offset < content.len() {
        let end = std::cmp::min(offset + CHUNK_SIZE, content.len());
        let chunk = &content[offset..end];

        if chunk.len() < MIN_CHARS {
            result.push_str(chunk);
        } else {
            let hash = ccr::compute_key(chunk.as_bytes());
            state.ccr_store.put(&hash, chunk);
            state.ccr_stored.fetch_add(1, Ordering::Relaxed);

            if state.add_markers {
                result.push_str(&ccr::marker_for(&hash));
            }
            saved += chunk.len();
        }
        offset = end;
    }

    if saved > 0 {
        state.total_tokens_saved.fetch_add((saved / 3) as u64, Ordering::Relaxed);
        Some(result)
    } else {
        None
    }
}
