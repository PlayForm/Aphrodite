//! `/retrieve` endpoint — resolve CCR markers to original content.

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};

use crate::proxy::AppState;

#[derive(Debug, Deserialize)]
pub struct RetrieveRequest {
    pub hash: Option<String>,
    pub query: Option<String>,
    pub path: Option<String>,
    #[serde(default)]
    pub offset: usize,
    #[serde(default)]
    pub limit: usize,
}

#[derive(Debug, Serialize)]
pub struct RetrieveResponse {
    pub found: bool,
    pub content: Option<String>,
    pub source: String,
    pub error: Option<String>,
}

pub async fn handle_retrieve(
    State(state): State<Arc<AppState>>,
    Json(req): Json<RetrieveRequest>,
) -> impl IntoResponse {
    let mut content = if let Some(path) = &req.path {
        match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                return (StatusCode::NOT_FOUND, Json(RetrieveResponse {
                    found: false, content: None,
                    source: format!("file:{}", path),
                    error: Some(format!("{}", e)),
                })).into_response();
            }
        }
    } else {
        let hash = match &req.hash {
            Some(h) => h.clone(),
            None => {
                return (StatusCode::BAD_REQUEST, Json(RetrieveResponse {
                    found: false, content: None, source: "none".into(),
                    error: Some("`hash` or `path` required".into()),
                })).into_response();
            }
        };

        match &state.ccr {
            Some(ccr) => {
                match ccr.get(&hash) {
                    Some(c) => {
                        state.ccr_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        c
                    }
                    None => {
                        state.ccr_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        return (StatusCode::NOT_FOUND, Json(RetrieveResponse {
                            found: false, content: None, source: "none".into(),
                            error: Some(format!("CCR entry not found: {}", hash)),
                        })).into_response();
                    }
                }
            }
            None => {
                state.ccr_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return (StatusCode::NOT_FOUND, Json(RetrieveResponse {
                    found: false, content: None, source: "none".into(),
                    error: Some(format!("CCR entry not found: {}", hash)),
                })).into_response();
            }
        }
    };

    // Apply query filter, then pagination
    content = filter_content(&content, req.query.as_deref());
    if req.limit > 0 {
        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        let start = req.offset.min(total);
        let end = (start + req.limit).min(total);
        content = lines[start..end].join("\n");
        if start > 0 || end < total {
            // Prepend range info when paginated
            content = format!("[lines {}-{}/{}]\n{}", start + 1, end, total, content);
        }
    }

    let source = if req.path.is_some() { format!("file:{}", req.path.as_ref().unwrap()) } else { "ccr".into() };
    Json(RetrieveResponse {
        found: true,
        content: Some(content),
        source,
        error: None,
    }).into_response()
}

fn filter_content(content: &str, query: Option<&str>) -> String {
    match query {
        Some(q) if !q.is_empty() => {
            let filtered: Vec<&str> = content.lines()
                .filter(|line| line.to_lowercase().contains(&q.to_lowercase()))
                .collect();
            if filtered.is_empty() {
                content.to_string()  // fallback to full content — don't silently return empty
            } else {
                filtered.join("\n")
            }
        }
        _ => content.to_string(),
    }
}
