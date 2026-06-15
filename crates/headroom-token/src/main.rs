//! headroom-token — Token-mode proxy binary.
//!
//! Standalone HTTP proxy that:
//! 1. Listens for OpenAI-compatible requests
//! 2. Forwards them to DeepSeek
//! 3. Compresses large tool outputs with CCR (SQLite-backed)
//! 4. Exposes `/retrieve` to resolve CCR markers
//! 5. Injects `headroom_retrieve` tool into compressed responses

use std::sync::Arc;

use axum::{
    extract::State,
    routing::{any, post},
    Json, Router,
};
use clap::Parser;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use headroom_token::config::Cli;
use headroom_token::proxy::{self};
use headroom_token::retrieve;

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

    tracing::info!(
        listen = %cli.listen,
        deepseek = %cli.deepseek_url,
        model = %cli.model,
        ccr_db = %cli.ccr_db_path.display(),
        ccr_ttl_s = cli.ccr_ttl_seconds,
        inject_tool = !cli.no_ccr_inject_tool,
        add_markers = !cli.no_ccr_marker,
        "headroom-token starting"
    );

    let state2 = state.clone();
    let app = Router::new()
        .route("/retrieve", post(retrieve::handle_retrieve))
        .route(
            "/stats",
            axum::routing::get(move || async move {
                Json(state2.stats_json())
            }),
        )
        .route("/health", axum::routing::get(|| async { "ok" }))
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
