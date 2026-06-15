//! aphrodite — Generic LLM proxy with CCR + tool relay.
//!
//! Works with any OpenAI-compatible API.

use std::sync::Arc;
use axum::{routing::{any, get, post}, Json, Router};
use clap::Parser;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use aphrodite::config::{Cli, ProxyMode};
use aphrodite::proxy::{self, health_check, handle_tool_relay, handle_ccr_create, handle_ccr_list};
use aphrodite::retrieve;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    let cli = Cli::parse();

    if let Some(parent) = cli.ccr_db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let state = Arc::new(proxy::build_state(&cli).await?);

    let mode_str = match cli.mode {
        ProxyMode::Cache => "cache",
        ProxyMode::Token => "token",
    };

    tracing::info!(
        listen = %cli.listen,
        mode = %mode_str,
        api_url = %cli.api_url,
        model = %cli.model,
        tool_relay = cli.tool_relay,
        "aphrodite starting"
    );

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/version", get(|| async { env!("CARGO_PKG_VERSION") }))
        .route("/stats", get({
            let s = state.clone();
            move || async move { Json(s.stats_json()) }
        }))
        .route("/retrieve", post(retrieve::handle_retrieve))
        .route("/tool/relay", post(handle_tool_relay))
        .route("/ccr/create", post(handle_ccr_create))
        .route("/ccr/list", get(handle_ccr_list))
        .route("/debug", get({
            let s = state.clone();
            move || async move {
                let stats = s.stats_json();
                let lat = &stats["latency_buckets_us"];
                let comp = &stats["compressions_by_type"];
                let errs = &stats["last_errors"];
                Json(serde_json::json!({
                    "proxy": "aphrodite",
                    "version": env!("CARGO_PKG_VERSION"),
                    "mode": stats["mode"],
                    "health": {
                        "requests_total": stats["requests"]["total"],
                        "requests_compressed": stats["requests"]["compressed"],
                        "ccr_hits": stats["ccr"]["hits"],
                        "ccr_created": stats["ccr"]["created"],
                    },
                    "latency": {
                        "lt_1ms": lat[0],
                        "lt_10ms": lat[1], 
                        "lt_100ms": lat[2],
                        "lt_1s": lat[3],
                        "gt_1s": lat[4],
                    },
                    "compression_by_type": comp,
                    "recent_errors": errs,
                }))
            }
        }))
        .route("/*path", any(proxy::proxy_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(cli.listen).await?;
    tracing::info!(addr = %listener.local_addr()?, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async { let _ = tokio::signal::ctrl_c().await; };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! { _ = ctrl_c => {}, _ = terminate => {} }
    tracing::info!("shutdown");
}
