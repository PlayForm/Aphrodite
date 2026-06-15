//! headroom-proxy — Cache+Token proxy binary with tool relay.
//!
//! Standalone HTTP proxy that:
//! 1. Listens for OpenAI-compatible requests
//! 2. Forwards to DeepSeek
//! 3. Compresses large tool outputs with CCR (SQLite-backed) in token mode
//! 4. Exposes /retrieve to resolve CCR markers
//! 5. Exposes /tool/relay for bidirectional Hermes communication
//! 6. Exposes /ccr/create for programmatic CCR entry creation
//! 7. Injects headroom_retrieve tool into compressed responses (token mode)
//!
//! Default ports: :9797 (cache mode), :9798 (token mode).

use std::sync::Arc;

use axum::{
    extract::State,
    routing::{any, get, post},
    Json, Router,
};
use clap::Parser;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use headroom_proxy::config::{Cli, ProxyMode};
use headroom_proxy::proxy::{self, handle_tool_relay, handle_ccr_create, handle_ccr_list};
use headroom_proxy::retrieve;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
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
        deepseek = %cli.deepseek_url,
        model = %cli.model,
        tool_relay = cli.tool_relay,
        notify_url = ?cli.notify_url,
        "headroom-proxy starting"
    );

    // Build router with all endpoints
    let state2 = state.clone();
    let mut app = Router::new()
        // Core endpoints
        .route("/health", get(|| async { "ok" }))
        .route("/stats", get(move || async move {
            Json(state2.stats_json())
        }))
        .route("/retrieve", post(retrieve::handle_retrieve))
        // Tool relay (enabled by --tool-relay flag)
        .route("/tool/relay", post(handle_tool_relay))
        // Programmatic CCR management
        .route("/ccr/create", post(handle_ccr_create))
        .route("/ccr/list", get(handle_ccr_list))
        // Catch-all proxy
        .route("/*path", any(proxy::proxy_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(cli.listen).await?;
    tracing::info!(addr = %listener.local_addr()?, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received");
}
