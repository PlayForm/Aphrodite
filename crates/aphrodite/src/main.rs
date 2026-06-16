//! aphrodite - Multi-proxy LLM proxy with CCR + tool relay.
//!
//! Two modes:
//! 1. Single proxy: `aphrodite --mode cache --listen :9797 --api-key KEY`
//! 2. Multi-proxy: `aphrodite` (reads aphrodite.toml, spawns all listeners)

use std::sync::Arc;
use axum::{extract::ConnectInfo, routing::{any, delete, get, post}, http::StatusCode, response::IntoResponse, Json, Router};
use clap::Parser;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use aphrodite::config::{Cli, MultiConfig, ProxyMode};
use aphrodite::proxy::{self, handle_tool_relay, handle_ccr_create, handle_ccr_list, handle_ccr_delete, health_check};
use aphrodite::retrieve;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Try multi-proxy config first, fall back to CLI
    let config_path = std::env::var("APHRODITE_CONFIG_PATH").unwrap_or_else(|_| "aphrodite.toml".to_string());
    let (proxies, log_compact): (Vec<(String, Cli)>, bool) = if std::path::Path::new(&config_path).exists() {
        let config = MultiConfig::load()?;
        let proxies: Vec<(String, Cli)> = config.proxies.iter().map(|p| {
            let cli = config.resolve(p);
            let name = p.name.clone().unwrap_or_else(|| format!("{}", cli.listen));
            (name, cli)
        }).collect();
        let log_compact = std::env::var("APHRODITE_LOG_COMPACT").is_ok();
        (proxies, log_compact)
    } else {
        let cli = Cli::parse();
        let log_compact = cli.log_compact || std::env::var("APHRODITE_LOG_COMPACT").is_ok();
        let name = format!("{}", cli.listen);
        (vec![(name, cli)], log_compact)
    };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = tracing_subscriber::registry().with(filter);
    if log_compact {
        subscriber
            .with(tracing_subscriber::fmt::layer().compact().with_target(false).without_time())
            .try_init()?;
    } else {
        subscriber
            .with(tracing_subscriber::fmt::layer())
            .try_init()?;
    }

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

    // Wait for first shutdown signal — triggers graceful shutdown in all proxies
    shutdown_signal().await;
    tracing::info!("shutdown signal received, draining in-flight requests...");

    // Clone abort handles so we can force-kill after handles are moved into join_all
    let abort_handles: Vec<_> = handles.iter().map(|h| h.abort_handle()).collect();

    // Listen for a second Ctrl+C to force immediate shutdown
    let second_signal = async {
        let _ = tokio::signal::ctrl_c().await;
        tracing::warn!("second shutdown signal received, forcing immediate shutdown");
    };
    tokio::pin!(second_signal);

    // 5-second drain timeout before forcing abort
    let drain_timeout = tokio::time::sleep(std::time::Duration::from_secs(5));
    tokio::pin!(drain_timeout);

    // Wait for graceful drain (via axum's with_graceful_shutdown), second signal, or timeout
    let drain_fut = futures::future::join_all(handles);
    tokio::pin!(drain_fut);

    tokio::select! {
        _ = &mut drain_fut => {
            tracing::info!("all proxy listeners completed gracefully");
        }
        _ = &mut drain_timeout => {
            tracing::info!("drain timeout (5s) reached, aborting remaining tasks");
            for h in &abort_handles {
                h.abort();
            }
            // Let aborted tasks settle
            let _ = (&mut drain_fut).await;
        }
        _ = &mut second_signal => {
            tracing::info!("force shutdown on second signal, aborting remaining tasks");
            for h in &abort_handles {
                h.abort();
            }
            let _ = (&mut drain_fut).await;
        }
    }

    Ok(())
}

async fn run_single(name: String, mut cli: Cli) -> anyhow::Result<()> {
    // Resolve relative ccr_db_path against the binary directory, not CWD.
    // This way the database path is stable regardless of where the process is launched from.
    if !cli.ccr_db_path.as_os_str().is_empty() && !cli.ccr_db_path.is_absolute() {
        if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let old = cli.ccr_db_path.display().to_string();
                cli.ccr_db_path = exe_dir.join(&cli.ccr_db_path);
                tracing::info!(
                    "resolved relative ccr_db_path from {} to {}",
                    old,
                    cli.ccr_db_path.display()
                );
            }
        }
    }
    if let Some(parent) = cli.ccr_db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let state = Arc::new(proxy::build_state(&cli).await?);
    let task_tracker = state.task_tracker.clone();

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
            move |ConnectInfo(addr): ConnectInfo<SocketAddr>| {
                let s = s.clone();
                async move {
                    if !addr.ip().is_loopback() {
                        return (StatusCode::FORBIDDEN, Json(serde_json::json!({
                            "error": "only loopback clients allowed"
                        }))).into_response();
                    }
                    let ok = s.client
                        .get(format!("{}/models", s.api_url.trim_end_matches('/')))
                        .header("Authorization", format!("Bearer {}", s.api_key))
                        .timeout(std::time::Duration::from_secs(5))
                        .send()
                        .await
                        .map(|r| r.status().is_success())
                        .unwrap_or(false);
                    Json(serde_json::json!({"upstream": ok})).into_response()
                }
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
                // LLM response cache
                if let Some(cache) = stats["cache"].as_object() {
                    out.push_str(&format!("aphrodite_cache_hits {}\n", cache["hits"]));
                    out.push_str(&format!("aphrodite_cache_misses {}\n", cache["misses"]));
                }
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

    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
    	.with_graceful_shutdown(shutdown_signal())
    	.await?;

    task_tracker.close();
    task_tracker.wait().await;
    tracing::debug!("all background tasks completed");

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
