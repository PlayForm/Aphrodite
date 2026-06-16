//! aphrodite - Multi-proxy LLM proxy with CCR + tool relay.
//!
//! Two modes:
//! 1. Single proxy: `aphrodite --mode cache --listen :9797 --api-key KEY`
//! 2. Multi-proxy: `aphrodite` (reads aphrodite.toml, spawns all listeners)

use std::sync::Arc;
use axum::{routing::{any, delete, get, post}, http::StatusCode, response::IntoResponse, Json, Router};
use clap::Parser;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use aphrodite::config::{Cli, MultiConfig, ProxyMode};
use aphrodite::proxy::{self, handle_tool_relay, handle_ccr_create, handle_ccr_list, handle_ccr_delete, health_check};
use aphrodite::retrieve;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    let compact = std::env::var("APHRODITE_LOG_COMPACT").is_ok();
    let subscriber = tracing_subscriber::registry().with(filter);
    if compact {
        subscriber
            .with(tracing_subscriber::fmt::layer().compact().with_target(false).without_time())
            .try_init()?;
    } else {
        subscriber
            .with(tracing_subscriber::fmt::layer())
            .try_init()?;
    }

    // Try multi-proxy config first, fall back to CLI
    let config_path = std::env::var("APHRODITE_CONFIG_PATH").unwrap_or_else(|_| "aphrodite.toml".to_string());
    let proxies: Vec<(String, Cli)> = if std::path::Path::new(&config_path).exists() {
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
        .route("/stats/db", get({
            let s = state.clone();
            move || async move {
                let mode = match s.mode {
                    ProxyMode::Cache => "cache",
                    ProxyMode::Token => "token",
                };
                match &s.ccr {
                    Some(ccr) => match ccr.stats_db() {
                        Some(stats) => Json(stats).into_response(),
                        None => (
                            StatusCode::OK,
                            Json(serde_json::json!({
                                "error": "stats_db not available for this backend",
                                "mode": mode,
                            })),
                        )
                            .into_response(),
                    },
                    None => (
                        StatusCode::OK,
                        Json(serde_json::json!({
                            "error": "CCR not enabled",
                            "mode": mode,
                        })),
                    )
                        .into_response(),
                }
            }
        }))
        // NOTE: No auth on /metrics - intentional for local-only deployments.
        // In production, add a reverse-proxy auth layer or firewall this endpoint.
        .route("/metrics", get({
            let s = state.clone();
            move || async move {
                let stats = s.stats_json();
                let mut out = String::new();
                let mode_str = stats["mode"].as_str().unwrap_or("unknown");
                out.push_str(&format!("aphrodite_requests_total{{mode=\"{}\"}} {}\n",
                    mode_str, stats["requests"]["total"]));
                out.push_str(&format!("aphrodite_requests_compressed{{mode=\"{}\"}} {}\n",
                    mode_str, stats["requests"]["compressed"]));
                out.push_str(&format!("aphrodite_tokens_saved {}\n", stats["tokens_saved"]));
                out.push_str(&format!("aphrodite_ccr_hits {}\n", stats["ccr"]["hits"]));
                out.push_str(&format!("aphrodite_ccr_misses {}\n", stats["ccr"]["misses"]));
                out.push_str(&format!("aphrodite_ccr_created {}\n", stats["ccr"]["created"]));
                out.push_str(&format!("aphrodite_tool_relay_calls {}\n", stats["tool_relay_calls"]));
                // Latency buckets
                if let Some(buckets) = stats["latency_buckets_us"].as_array() {
                    for (i, v) in buckets.iter().enumerate() {
                        let le = match i { 0=>"0.001", 1=>"0.01", 2=>"0.1", 3=>"1.0", 4=>"10.0", _=>"+Inf"};
                        out.push_str(&format!("aphrodite_latency_seconds_bucket{{le=\"{}\"}} {}\n", le, v));
                    }
                }
                if let Some(ratio) = stats["compression_ratio_ema"].as_f64() {
                    out.push_str(&format!("aphrodite_compression_ratio_ema {:.2}\n", ratio));
                }
                (StatusCode::OK, [(axum::http::header::CONTENT_TYPE, "text/plain; version=0.0.4")], out)
            }
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
        .route("/ccr/{hash}", delete(handle_ccr_delete))
        .route("/favicon.ico", get(|| async { StatusCode::NOT_FOUND }))
        .route("/robots.txt", get(|| async { "User-agent: *\nDisallow: /\n" }))
        .route("/", get(|| async { Json(serde_json::json!({"proxy": "aphrodite", "version": env!("CARGO_PKG_VERSION")})) }))
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
}
