//! aphrodite — Multi-proxy LLM proxy with CCR + tool relay.
//!
//! Two modes:
//! 1. Single proxy: `aphrodite --mode cache --listen :9797 --api-key KEY`
//! 2. Multi-proxy: `aphrodite` (reads aphrodite.toml, spawns all listeners)

use std::sync::Arc;
use axum::{routing::{any, get, post}, Json, Router};
use clap::Parser;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use aphrodite::config::{Cli, MultiConfig, ProxyMode};
use aphrodite::proxy::{self, handle_tool_relay, handle_ccr_create, handle_ccr_list, health_check};
use aphrodite::retrieve;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    // Try multi-proxy config first, fall back to CLI
    let proxies: Vec<(String, Cli)> = if std::path::Path::new("aphrodite.toml").exists() {
        let config = MultiConfig::load()?;
        config.proxies.iter().map(|p| {
            let cli = config.resolve(p);
            let name = p.name.clone().unwrap_or_else(|| format!("{}", cli.listen));
            (name, cli)
        }).collect()
    } else {
        let cli = Cli::parse();
        let name = format!("{}", cli.listen);
        vec![(name, cli)]
    };

    tracing::info!("starting {} proxy listener(s)", proxies.len());

    let mut handles = Vec::new();
    for (name, cli) in proxies {
        let handle = tokio::spawn(async move {
            if let Err(e) = run_single(name, cli).await {
                tracing::error!(%e, "proxy listener failed");
            }
        });
        handles.push(handle);
    }

    // Wait for shutdown signal
    shutdown_signal().await;
    tracing::info!("shutdown signal received, stopping all listeners");

    for h in handles {
        h.abort();
    }

    Ok(())
}

async fn run_single(name: String, cli: Cli) -> anyhow::Result<()> {
    if let Some(parent) = cli.ccr_db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let state = Arc::new(proxy::build_state(&cli).await?);

    let mode_str = match cli.mode {
        ProxyMode::Cache => "cache",
        ProxyMode::Token => "token",
    };

    tracing::info!(
        name = %name,
        listen = %cli.listen,
        mode = %mode_str,
        api_url = %cli.api_url,
        model = %cli.model,
        tool_relay = cli.tool_relay,
        "proxy starting"
    );

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/health/upstream", get({
            let s = state.clone();
            move || async move {
                let ok = s.client
                    .get(format!("{}/models", s.api_url.trim_end_matches('/')))
                    .header("Authorization", format!("Bearer {}", s.api_key))
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                    .map(|r| r.status().is_success())
                    .unwrap_or(false);
                Json(serde_json::json!({"upstream": ok}))
            }
        }))
        .route("/version", get(|| async { env!("CARGO_PKG_VERSION") }))
        .route("/stats", get({
            let s = state.clone();
            move || async move { Json(s.stats_json()) }
        }))
        .route("/history", get({
            let s = state.clone();
            move || async move {
                Json(s.request_history.lock().map(|v| v.clone()).unwrap_or_default())
            }
        }))
        .route("/retrieve", post(retrieve::handle_retrieve))
        .route("/tool/relay", post(handle_tool_relay))
        .route("/ccr/create", post(handle_ccr_create))
        .route("/ccr/list", get(handle_ccr_list))
        .route("/*path", any(proxy::proxy_handler))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(cli.listen).await?;
    tracing::info!(addr = %listener.local_addr()?, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c().await.ok();
        })
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
}
