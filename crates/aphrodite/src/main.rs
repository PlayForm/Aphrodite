//! aphrodite - Multi-proxy LLM proxy with CCR + tool relay.
//!
//! Two modes:
//! 1. Single proxy: `aphrodite --mode cache --listen :9797 --api-key KEY`
//! 2. Multi-proxy: `aphrodite` (reads aphrodite.toml, spawns all listeners)

use std::sync::Arc;
use axum::{
	extract::{ConnectInfo, DefaultBodyLimit, Request},
	http::StatusCode,
	middleware::{self, Next},
	response::{IntoResponse, Json},
	routing::{any, delete, get, post},
	Router,
};
use clap::Parser;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use aphrodite::config::{Cli, MultiConfig, ProxyMode};
use aphrodite::proxy::{self, handle_tool_relay, handle_ccr_create, handle_ccr_list, handle_ccr_delete, handle_ccr_reload, health_check};
use aphrodite::retrieve;

fn main() -> anyhow::Result<()> {
	// Handle --version early — clap version attribute is only wired through
	// Cli::parse() which is skipped when aphrodite.toml exists (multi-proxy path).
	// This ensures --version always works regardless of config state.
	let args: Vec<String> = std::env::args().collect();
	if args.iter().any(|a| a == "--version" || a == "-V") {
		println!("aphrodite v{}", option_env!("APHRODITE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION")));
		return Ok(());
	}

	// Worker thread count  -  I/O-bound proxy needs more than CPU cores.
	// Default: 4× CPU or 32 minimum. Override: APHRODITE_WORKER_THREADS.
	let worker_threads = std::env::var("APHRODITE_WORKER_THREADS")
		.ok()
		.and_then(|v| v.parse::<usize>().ok())
		.unwrap_or_else(|| {
			let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8);
			(cpus * 4).max(32)
		});
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.worker_threads(worker_threads)
		.enable_all()
		.build()
		.expect("tokio runtime");
	runtime.block_on(run())
}

async fn run() -> anyhow::Result<()> {
	// Try multi-proxy config first, fall back to CLI
	let config_path = std::env::var("APHRODITE_CONFIG_PATH").unwrap_or_else(|_| "aphrodite.toml".to_string());
	let (proxies, log_compact): (Vec<(String, Cli)>, bool) = if std::path::Path::new(&config_path).exists() {
		let config = MultiConfig::load(&config_path)?;
		let proxies: Vec<(String, Cli)> = config
			.proxies
			.iter()
			.map(|p| {
				let cli = config.resolve(p)?;
				let name = p.name.clone().unwrap_or_else(|| format!("{}", cli.listen));
				Ok((name, cli))
			})
			.collect::<anyhow::Result<Vec<_>>>()?;
		let log_compact = std::env::var("APHRODITE_LOG_COMPACT").is_ok();
		(proxies, log_compact)
	} else {
		let cli = Cli::parse();
		let log_compact = cli.log_compact || std::env::var("APHRODITE_LOG_COMPACT").is_ok();
		let name = format!("{}", cli.listen);
		(vec![(name, cli)], log_compact)
	};

	let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
	let subscriber = tracing_subscriber::registry().with(filter);
	if log_compact {
		subscriber
			.with(tracing_subscriber::fmt::layer().compact().with_target(false).without_time())
			.try_init()?;
	} else {
		subscriber.with(tracing_subscriber::fmt::layer()).try_init()?;
	}

	tracing::info!(
		"aphrodite v{} ({}{}) • {} • {}",
		option_env!("APHRODITE_VERSION").unwrap_or("?"),
		option_env!("APHRODITE_GIT_HASH").unwrap_or("?"),
		option_env!("APHRODITE_PROFILE").map(|p| format!(", {p}")).unwrap_or_default(),
		option_env!("APHRODITE_BUILD_DATE").unwrap_or("?"),
		option_env!("APHRODITE_TARGET").unwrap_or("?"),
	);

	tracing::info!("starting {} proxy listener(s)", proxies.len());

		// ── Spawn config file watcher for hot-reload ──────────────────
		let watch_path = {
			let p = std::path::PathBuf::from(&config_path);
			if p.is_relative() {
				std::env::current_dir().unwrap_or_default().join(&p)
			} else {
				p
			}
		};
		let watch_path_str = watch_path.to_string_lossy().to_string();
		tokio::spawn(async move {
		use notify::{Event, EventKind, RecursiveMode, Watcher};
		let (tx, mut rx) = tokio::sync::mpsc::channel(16);
		let mut watcher = match notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
			if let Ok(event) = res {
				let is_modify = matches!(event.kind, EventKind::Modify(_));
				if is_modify && event.paths.iter().any(|p| p.to_string_lossy().contains("aphrodite.toml")) {
					let _ = tx.try_send(());
				}
			}
		}) {
			Ok(w) => w,
			Err(e) => {
				tracing::warn!("failed to create config watcher: {e}");
				return;
			}
		};
		let watch_dir = std::path::Path::new(&watch_path).parent()
			.unwrap_or(std::path::Path::new("."));
		if let Err(e) = watcher.watch(watch_dir, RecursiveMode::NonRecursive) {
			tracing::warn!("failed to start config watcher: {e}");
			return;
		}
		tracing::info!(path = %watch_path_str, "config file watcher active");
		loop {
			if rx.recv().await.is_some() {
				// Debounce: wait 500ms for writes to settle
				tokio::time::sleep(std::time::Duration::from_millis(500)).await;
				// Drain any additional events accumulated during debounce
				while rx.try_recv().is_ok() {}
				match aphrodite::config::MultiConfig::load(&watch_path_str) {
					Ok(config) => {
						let comp = config.compression.as_ref();
						tracing::info!(
							path = %watch_path_str,
							auto_expand_limit = comp.and_then(|c| c.auto_expand_limit).unwrap_or(0),
							engine_threshold = comp.and_then(|c| c.engine_threshold_pct).unwrap_or(0),
							"? config reloaded — proxy will use new values; plugin reloads independently"
						);
					}
					Err(e) => {
						tracing::warn!(error = %e, "failed to reload config on file change");
					}
				}
			}
		}
	});

	// ── Shared shutdown watch channel ────────────────────────────────
	// Initial value false; main() sets true on first Ctrl+C/SIGTERM,
	// each run_single() task waits on the receiver for graceful_shutdown.
	let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

	let mut handles = Vec::new();
	for (name, cli) in proxies {
		let rx = shutdown_rx.clone();
		let handle = tokio::spawn(async move {
			if let Err(e) = run_single(name, cli, rx).await {
				tracing::error!(%e, "proxy listener failed");
			}
		});
		handles.push(handle);
	}
	// Drop the original sender so run_single receivers get a RecvError::Closed
	// when main() finishes (SIGTERM alternative: drop triggers changed() wakeup).
	drop(shutdown_rx);

	// Wait for first shutdown signal - triggers graceful shutdown in all proxies
	shutdown_signal().await;
	let _ = shutdown_tx.send(true);
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

	// Wait for graceful drain (via axum's with_graceful_shutdown), second signal, or timeout.
	// NOTE: do NOT re-await drain_fut after select! - it was polled by select! and
	// re-awaiting would be double-poll UB. Abort handles handle the remaining tasks.
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
		}
		_ = &mut second_signal => {
			tracing::info!("force shutdown on second signal, aborting remaining tasks");
			for h in &abort_handles {
				h.abort();
			}
		}
	}

	Ok(())
}

async fn run_single(
	name: String,
	mut cli: Cli,
	mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> anyhow::Result<()> {
	// Resolve relative ccr_db_path against the binary directory, not CWD.
	// This way the database path is stable regardless of where the process is launched from.
	if !cli.ccr_db_path.as_os_str().is_empty() && !cli.ccr_db_path.is_absolute() {
		if let Ok(exe_path) = std::env::current_exe() {
			if let Some(exe_dir) = exe_path.parent() {
				let old = cli.ccr_db_path.display().to_string();
				cli.ccr_db_path = exe_dir.join(&cli.ccr_db_path);
				tracing::info!("resolved relative ccr_db_path from {} to {}", old, cli.ccr_db_path.display());
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

	// Restricted routes - loopback-only enforcement for everything except /health
	let restricted = Router::new()
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
                out.push_str(&format!("aphrodite_requests_compressed_total{{mode=\"{}\"}} {}\n",
                    mode_str, stats["requests"]["compressed"]));
                out.push_str(&format!("aphrodite_tokens_saved_total {}\n", stats["tokens_saved"]));
                out.push_str(&format!("aphrodite_ccr_hits_total {}\n", stats["ccr"]["hits"]));
                out.push_str(&format!("aphrodite_ccr_misses_total {}\n", stats["ccr"]["misses"]));
                out.push_str(&format!("aphrodite_ccr_created_total {}\n", stats["ccr"]["created"]));
                out.push_str(&format!("aphrodite_tool_relay_calls_total {}\n", stats["tool_relay_calls"]));
                // LLM response cache
                if let Some(cache) = stats["cache"].as_object() {
                    out.push_str(&format!("aphrodite_cache_hits_total {}\n", cache["hits"]));
                    out.push_str(&format!("aphrodite_cache_misses_total {}\n", cache["misses"]));
                }
                // Latency buckets
                if let Some(buckets) = stats["latency_buckets_us"].as_array() {
                    let mut total_count: u64 = 0;
                    for (i, v) in buckets.iter().enumerate() {
                        let le = match i { 0=>"0.001", 1=>"0.01", 2=>"0.1", 3=>"1.0", 4=>"10.0", _=>"+Inf"};
                        let count = v.as_u64().unwrap_or(0);
                        total_count += count;
                        out.push_str(&format!("aphrodite_latency_seconds_bucket{{le=\"{}\"}} {}\n", le, total_count));
                    }
                    out.push_str(&format!("aphrodite_latency_seconds_count {}\n", total_count));
                    if let Some(total_us) = stats["total_latency_micros"].as_u64() {
                        out.push_str(&format!("aphrodite_latency_seconds_sum {:.6}\n", total_us as f64 / 1_000_000.0));
                    }
                }
                if let Some(ratio) = stats["compression_ratio_ema"].as_f64() {
                    out.push_str(&format!("aphrodite_compression_ratio_ema {:.2}\n", ratio));
                }
                // Inline CCR
                if let Some(icc) = stats["inline_ccr"].as_object() {
                    if let Some(h) = icc["hits"].as_u64() { out.push_str(&format!("aphrodite_inline_ccr_hits_total {h}\n")); }
                    if let Some(m) = icc["misses"].as_u64() { out.push_str(&format!("aphrodite_inline_ccr_misses_total {m}\n")); }
                }
                // Tool relay success/failure
                if let Some(tr) = stats["tool_relay"].as_object() {
                    if let Some(s) = tr["success"].as_u64() { out.push_str(&format!("aphrodite_tool_relay_success_total {s}\n")); }
                    if let Some(f) = tr["failure"].as_u64() { out.push_str(&format!("aphrodite_tool_relay_failure_total {f}\n")); }
                }
                // Notify success/failure
                if let Some(n) = stats["notify"].as_object() {
                    if let Some(s) = n["success"].as_u64() { out.push_str(&format!("aphrodite_notify_success_total {s}\n")); }
                    if let Some(f) = n["failure"].as_u64() { out.push_str(&format!("aphrodite_notify_failure_total {f}\n")); }
                }
                // Upstream errors
                if let Some(ue) = stats["upstream_errors"].as_object() {
                    if let Some(c) = ue["4xx"].as_u64() { out.push_str(&format!("aphrodite_upstream_errors_total{{code=\"4xx\"}} {c}\n")); }
                    if let Some(c) = ue["5xx"].as_u64() { out.push_str(&format!("aphrodite_upstream_errors_total{{code=\"5xx\"}} {c}\n")); }
                    if let Some(t) = ue["timeouts"].as_u64() { out.push_str(&format!("aphrodite_upstream_timeouts_total {t}\n")); }
                }
                // CCR store info
                if let Some(cs) = stats["ccr_store"].as_object() {
                    if let Some(e) = cs["entries"].as_u64() { out.push_str(&format!("aphrodite_ccr_store_entries {e}\n")); }
                    if let Some(b) = cs["bytes_approx"].as_u64() { out.push_str(&format!("aphrodite_ccr_store_bytes {b}\n")); }
                }
                // Body bytes
                if let Some(bb) = stats["body_bytes"].as_object() {
                    if let Some(r) = bb["request"].as_u64() { out.push_str(&format!("aphrodite_request_body_bytes_total {r}\n")); }
                    if let Some(r) = bb["response"].as_u64() { out.push_str(&format!("aphrodite_response_body_bytes_total {r}\n")); }
                }
                // Upstream latency
                if let Some(ul) = stats["upstream_latency_micros"].as_u64() {
                    out.push_str(&format!("aphrodite_upstream_latency_seconds_total {:.6}\n", ul as f64 / 1_000_000.0));
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
        .route("/reload", post(handle_ccr_reload))
        .route("/favicon.ico", get(|| async { StatusCode::NOT_FOUND }))
        .route("/robots.txt", get(|| async { "User-agent: *\nDisallow: /\n" }))
        .route("/", get(|| async {
            Json(serde_json::json!({
                "proxy": "aphrodite",
                "version": env!("CARGO_PKG_VERSION"),
                "git_hash": option_env!("APHRODITE_GIT_HASH"),
                "build_date": option_env!("APHRODITE_BUILD_DATE"),
                "target": option_env!("APHRODITE_TARGET"),
                "profile": option_env!("APHRODITE_PROFILE"),
            }))
        }))
        .route("/{*path}", any(proxy::proxy_handler))
        // Loopback enforcement layer on all non-/health routes
        .layer(middleware::from_fn(loopback_only))
        // 1 MB body limit on restricted routes
        .layer(DefaultBodyLimit::max(1024 * 1024));

	// Public route (no loopback enforcement) merged with restricted routes
	let app = Router::new()
		.route("/health", get(health_check))
		.merge(restricted)
		.layer(CorsLayer::permissive())
		.with_state(state);

	let listener = tokio::net::TcpListener::bind(cli.listen).await?;
	tracing::info!(addr = %listener.local_addr()?, "listening");

	// Graceful shutdown triggered by shared watch channel - fires when main()
	// calls shutdown_tx.send(true) after OS signal, or when sender is dropped.
	let shutdown_fut = async move {
		let _ = shutdown_rx.changed().await;
	};
	let serve_result = axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
		.with_graceful_shutdown(shutdown_fut)
		.await;

	// Close task tracker BEFORE propagating serve error so background tasks
	// are not leaked when serve fails (e.g. epoll registration error).
	task_tracker.close();
	task_tracker.wait().await;
	tracing::debug!("all background tasks completed");

	serve_result?;

	Ok(())
}

/// Middleware that rejects non-loopback clients on all routes except /health.
/// /health is intentionally exempt so external load-balancer probes work.
async fn loopback_only(
	ConnectInfo(addr): ConnectInfo<SocketAddr>,
	request: Request,
	next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
	if !addr.ip().is_loopback() {
		return Err((
			StatusCode::FORBIDDEN,
			Json(serde_json::json!({"error": "only loopback clients allowed"})),
		));
	}
	Ok(next.run(request).await)
}

async fn shutdown_signal() {
	let ctrl_c = async {
		let _ = tokio::signal::ctrl_c().await;
	};
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
