//! `/retrieve` endpoint — resolve CCR markers to original content.
//!
//! No governor auth. Accepts `{"hash": "...", "query": "..."}` and returns
//! `{"original_content": "...", "source": "ccr"}` or 404.
//! When `path` is provided, reads the file directly from disk.

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;

use crate::proxy::AppState;

#[derive(Deserialize)]
pub struct RetrieveRequest {
    pub hash: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
}

#[derive(Serialize)]
pub struct RetrieveResponse {
    pub original_content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub original_tokens: Option<usize>,
}

pub async fn handle_retrieve(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RetrieveRequest>,
) -> impl IntoResponse {
    // Path-based: read file directly from disk (bypasses proxy)
    if let Some(path) = &req.path {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            match std::fs::read_to_string(trimmed) {
                Ok(content) => {
                    tracing::info!(path = %trimmed, len = content.len(), "file retrieve");
                    return (StatusCode::OK, Json(RetrieveResponse {
                        original_content: content,
                        source: Some("local".to_string()),
                        original_tokens: None,
                    })).into_response();
                }
                Err(e) => {
                    tracing::warn!(path = %trimmed, error = %e, "file retrieve failed");
                    return (StatusCode::NOT_FOUND, Json(serde_json::json!({
                        "error": format!("Cannot read file: {}", e)
                    }))).into_response();
                }
            }
        }
    }

    let hash = req.hash.trim().to_string();
    if hash.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "missing hash"
        }))).into_response();
    }

    match state.ccr_store.get(&hash) {
        Some(content) => {
            state.ccr_hits.fetch_add(1, Ordering::Relaxed);
            tracing::info!(hash = %hash, len = content.len(), "ccr retrieve hit");

            // Apply BM25 query filter if provided
            let final_content = if let Some(query) = &req.query {
                if !query.trim().is_empty() {
                    filter_by_query(&content, query.trim())
                } else {
                    content
                }
            } else {
                content
            };

            (StatusCode::OK, Json(RetrieveResponse {
                original_content: final_content,
                source: Some("ccr".to_string()),
                original_tokens: None,
            })).into_response()
        }
        None => {
            state.ccr_misses.fetch_add(1, Ordering::Relaxed);
            tracing::warn!(hash = %hash, "ccr retrieve miss");
            (StatusCode::NOT_FOUND, Json(serde_json::json!({
                "error": "Content not found: expired (TTL passed) or proxy restarted. Re-run the original command to regenerate the data."
            }))).into_response()
        }
    }
}

/// Simple BM25-like keyword search over content lines.
fn filter_by_query(content: &str, query: &str) -> String {
    let terms: Vec<&str> = query.split_whitespace().collect();
    if terms.is_empty() {
        return content.to_string();
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut scored: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| {
            let lower = line.to_lowercase();
            let score = terms.iter().filter(|t| lower.contains(&t.to_lowercase())).count();
            (score, *line)
        })
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0));

    let relevant: Vec<&str> = scored
        .into_iter()
        .filter(|(score, _)| *score > 0)
        .map(|(_, line)| line)
        .collect();

    if relevant.is_empty() {
        content.to_string()
    } else {
        relevant.join("\n")
    }
}
