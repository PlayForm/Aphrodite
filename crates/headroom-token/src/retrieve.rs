//! `/retrieve` endpoint — resolve CCR markers to original content.

use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};

use headroom_core::ccr::CcrStore;
use crate::proxy::AppState;

#[derive(Debug, Deserialize)]
pub struct RetrieveRequest {
    pub hash: Option<String>,
    pub query: Option<String>,
    pub path: Option<String>,
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
    // Path-based file read
    if let Some(path) = &req.path {
        match std::fs::read_to_string(path) {
            Ok(content) => {
                return Json(RetrieveResponse {
                    found: true,
                    content: Some(filter_content(&content, req.query.as_deref())),
                    source: format!("file:{}", path),
                    error: None,
                }).into_response();
            }
            Err(e) => {
                return (StatusCode::NOT_FOUND, Json(RetrieveResponse {
                    found: false, content: None,
                    source: format!("file:{}", path),
                    error: Some(format!("{}", e)),
                })).into_response();
            }
        }
    }

    let hash = match &req.hash {
        Some(h) => h.clone(),
        None => {
            return (StatusCode::BAD_REQUEST, Json(RetrieveResponse {
                found: false, content: None, source: "none".into(),
                error: Some("`hash` or `path` required".into()),
            })).into_response();
        }
    };

    if let Some(ccr) = &state.ccr {
        match ccr.get(&hash) {
            Some(content) => {
                state.ccr_hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                return Json(RetrieveResponse {
                    found: true,
                    content: Some(filter_content(&content, req.query.as_deref())),
                    source: "ccr".into(),
                    error: None,
                }).into_response();
            }
            None => {
                state.ccr_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
    }

    state.ccr_misses.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    (
        StatusCode::NOT_FOUND,
        Json(RetrieveResponse {
            found: false, content: None, source: "none".into(),
            error: Some(format!("CCR entry not found: {}", hash)),
        }),
    )
        .into_response()
}

fn filter_content(content: &str, query: Option<&str>) -> String {
    match query {
        Some(q) if !q.is_empty() => {
            content.lines()
                .filter(|line| line.to_lowercase().contains(&q.to_lowercase()))
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => content.to_string(),
    }
}
