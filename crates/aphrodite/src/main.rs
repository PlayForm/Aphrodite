//! aphrodite — Cache+Token Chat Completions proxy.
//!
//! Standalone HTTP proxy that:
//! 1. Listens for Chat Completions API requests (POST /v1/chat/completions)
//! 2. Forwards to DeepSeek
//! 3. Compresses large tool outputs with CCR
//! 4. Exposes /retrieve for CCR lookup
//! 5. Exposes /tool/relay for bidirectional Hermes communication
//! 6. Exposes /ccr/create + /ccr/list for programmatic CCR
//!
//! Cache mode (:9797): in-memory, >8KB threshold, preview kept.
//! Token mode (:9798): SQLite, >1KB threshold, tool injection.

use std::sync::Arc;

use axum::{
    routing::{any, get, post},
    Json, Router,
};
use clap::Parser;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use aphrodite::config::{Cli, ProxyMode};
use aphrodite::proxy::{self, handle_tool_relay, handle_ccr_create, handle_ccr_list};
use aphrodite::retrieve;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    let cli = Cli::parse();

    // Dev mode: also write to /tmp/aphrodite-dev.log
    if cli.dev {
        let dev_log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/aphrodite-dev.log")?;
        let (writer, _guard) = tracing_appender::non_blocking(dev_log);
        tracing_subscriber::fmt()
            .with_writer(writer)
            .with_env_filter("aphrodite=debug")
            .try_init()
            .ok();
    }

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
        "aphrodite starting"
    );

    let mut app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/stats", get({
            let s = state.clone();
            move || async move { Json(s.stats_json()) }
        }))
        .route("/retrieve", post(retrieve::handle_retrieve))
        .route("/tool/relay", post(handle_tool_relay))
        .route("/ccr/create", post(handle_ccr_create))
        .route("/ccr/list", get(handle_ccr_list))
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
