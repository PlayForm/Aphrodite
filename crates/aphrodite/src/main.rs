//! aphrodite - Multi-proxy LLM proxy with CCR + tool relay.
//!
//! Two modes:
//! 1. Single proxy: `aphrodite --mode cache --listen :9797 --api-key KEY`
//! 2. Multi-proxy: `aphrodite` (reads aphrodite.toml, spawns all listeners)

use std::{
	net::SocketAddr,
	sync::{Arc, atomic::Ordering},
};

use anyhow::Context;
use axum::{
	Router,
	extract::{ConnectInfo, DefaultBodyLimit, Request},
	http::StatusCode,
	middleware::{self, Next},
	response::{IntoResponse, Json},
	routing::{any, delete, get, post},
};
use clap::Parser;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use aphrodite::{
	config::{Cli, Command, MultiConfig, ProxyMode, SetupArgs},
	proxy::{
		self, handle_ccr_create, handle_ccr_delete, handle_ccr_list, handle_ccr_reload, handle_tool_relay, health_check,
	},
	retrieve, setup,
};

fn main() -> anyhow::Result<()> {
	// Handle --version / --help early to avoid starting the runtime.
	let args: Vec<String> = std::env::args().collect();
	if args.iter().any(|a| a == "--version" || a == "-V") {
		println!(
			"aphrodite v{}",
			option_env!("APHRODITE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"))
		);
		return Ok(());
	}
	if args.get(1).map(String::as_str) == Some("--help") || args.get(1).map(String::as_str) == Some("-h") {
		// Cli::parse() prints help/errors and exits the process internally.
		Cli::parse();
		return Ok(());
	}

	// ── Handle `aphrodite setup` subcommand ──
	// Parse early to check for setup before building the tokio runtime.
	if args.get(1).map(String::as_str) == Some("setup") {
		let cli = Cli::parse();
		if let Some(Command::Setup { api_key, api_url, model, cache_port, token_port, no_launch, force }) = cli.command
		{
			let setup_args = SetupArgs { api_key, api_url, model, cache_port, token_port, no_launch, force };
			match setup::run(&setup_args) {
				Ok(()) => {
					if setup_args.no_launch {
						// --no-launch: setup done, don't start proxy.
						return Ok(());
					}
					// Default: setup complete, drop through to start proxy
					println!("setup complete, starting proxy...");
				},
				Err(e) => {
					eprintln!("setup failed: {e}");
					std::process::exit(1);
				},
			}
		}
	}

	// Worker thread count - I/O-bound proxy needs more than CPU cores.
	// Default: 4× CPU or 32 minimum. Override: APHRODITE_WORKER_THREADS.
	// F15: warn (via eprintln, not tracing - the subscriber isn't initialized
	// this early, see `run()`'s matching comment) on a malformed value
	// instead of silently falling back, mirroring `apply_port_override`.
	let worker_threads = match std::env::var("APHRODITE_WORKER_THREADS") {
		Ok(v) => match v.parse::<usize>() {
			Ok(n) => n,
			Err(_) => {
				eprintln!("APHRODITE_WORKER_THREADS={v:?} is not a valid number; using the computed default");
				let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8);
				(cpus * 4).max(32)
			},
		},
		Err(_) => {
			let cpus = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(8);
			(cpus * 4).max(32)
		},
	};
	let runtime = tokio::runtime::Builder::new_multi_thread()
		.worker_threads(worker_threads)
		.enable_all()
		.build()
		.expect("tokio runtime");
	runtime.block_on(run())
}

async fn run() -> anyhow::Result<()> {
	// Try multi-proxy config first, fall back to CLI. `log_compact` is
	// resolved WITHOUT touching `MultiConfig::resolve()` so the tracing
	// subscriber can be initialized before that runs - `resolve()` emits
	// `tracing::info!`/`tracing::warn!` diagnostics (mode fallback, port
	// overrides, timeout clamping) that would otherwise fire against no
	// registered subscriber and be silently dropped, defeating the whole
	// point of adding them.
	// F6: `aphrodite setup` writes its generated config to
	// `~/.hermes/aphrodite/aphrodite.toml`, but this used to only ever look
	// at `./aphrodite.toml` (or an explicit `APHRODITE_CONFIG_PATH`) - a
	// setup run from any directory without its own `aphrodite.toml` fell
	// straight into CLI-parse mode, where `--api-key`/`APHRODITE_API_KEY` is
	// required, and the templated config (ports, model, api_url) was never
	// read. Only falls back when `APHRODITE_CONFIG_PATH` was NOT explicitly
	// set - an explicit override that doesn't exist should surface as
	// CLI-fallback (or a clear error), not silently redirect elsewhere.
	let explicit_config_path = std::env::var("APHRODITE_CONFIG_PATH").ok();
	let config_path = explicit_config_path.clone().unwrap_or_else(|| "aphrodite.toml".to_string());
	let use_multi_config = std::path::Path::new(&config_path).exists();
	let (config_path, use_multi_config) = if use_multi_config || explicit_config_path.is_some() {
		(config_path, use_multi_config)
	} else {
		match dirs::home_dir().map(|h| h.join(".hermes").join("aphrodite").join("aphrodite.toml")) {
			Some(p) if p.exists() => (p.to_string_lossy().into_owned(), true),
			_ => (config_path, false),
		}
	};
	let (multi_config, cli_fallback, log_compact) = if use_multi_config {
		let config = MultiConfig::load(&config_path)?;
		let log_compact = aphrodite::config::env_bool("APHRODITE_LOG_COMPACT");
		(Some(config), None, log_compact)
	} else {
		let cli = Cli::parse();
		let log_compact = cli.log_compact || aphrodite::config::env_bool("APHRODITE_LOG_COMPACT");
		(None, Some(cli), log_compact)
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

	// The `[compression]` table is a single top-level section shared by every
	// `[[proxies]]` entry (report 07 F2/T15) - each listener's `AppState`
	// resolves its own cache-vs-token threshold from the same table via
	// `resolve_thresholds`.
	let compression = multi_config.as_ref().and_then(|c| c.compression.clone());

	let proxies: Vec<(String, Cli)> = if let Some(config) = multi_config {
		config
			.proxies
			.iter()
			.map(|p| {
				let cli = config.resolve(p)?;
				let name = p.name.clone().unwrap_or_else(|| format!("{}", cli.listen));
				Ok((name, cli))
			})
			.collect::<anyhow::Result<Vec<_>>>()?
	} else {
		let cli = cli_fallback.expect("cli_fallback set when use_multi_config is false");
		// A standalone (no aphrodite.toml) proxy launch needs an upstream key.
		// `api_key` is optional at parse time so `setup`/keyless invocations
		// don't require it; enforce it here for the proxy path specifically,
		// with a clear error instead of silently forwarding an empty Bearer.
		if cli.api_key.trim().is_empty() {
			anyhow::bail!(
				"no API key configured - set APHRODITE_API_KEY env var or pass --api-key, or run `aphrodite setup`"
			);
		}
		let name = format!("{}", cli.listen);
		vec![(name, cli)]
	};

	tracing::info!(
		"aphrodite v{} ({}{}) • {} • {}",
		option_env!("APHRODITE_VERSION").unwrap_or("?"),
		option_env!("APHRODITE_GIT_HASH").unwrap_or("?"),
		option_env!("APHRODITE_PROFILE").map(|p| format!(", {p}")).unwrap_or_default(),
		option_env!("APHRODITE_BUILD_DATE").unwrap_or("?"),
		option_env!("APHRODITE_TARGET").unwrap_or("?"),
	);

	tracing::info!("starting {} proxy listener(s)", proxies.len());

	// ── Shared shutdown watch channel ────────────────────────────────
	// Initial value false; main() sets true on first Ctrl+C/SIGTERM,
	// each run_single() task waits on the receiver for graceful_shutdown.
	let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

	// F9: bind every listener BEFORE spawning any proxy task. Previously
	// each `run_single` bound its own port deep inside its spawned task, so
	// a bind failure (port already in use - a stale instance, a conflict
	// between the cache/token pair) only logged an error and left that one
	// listener silently dead while the other kept serving - the process
	// exited 0 and looked healthy. Binding here means a failure aborts
	// startup entirely, loudly, before any listener goes live.
	//
	// `AppState` is also built here now, not inside `run_single` (report 07
	// F4/T15) - the config-file watcher below needs every listener's
	// `Arc<AppState>` up front so a file change can actually apply new
	// compression thresholds to the live proxies, not just re-parse and log.
	let mut bound = Vec::with_capacity(proxies.len());
	for (name, mut cli) in proxies {
		// Resolve relative ccr_db_path against the binary directory, not CWD.
		// This way the database path is stable regardless of where the
		// process is launched from. (Moved here from `run_single` so
		// `build_state` can run before spawning.)
		if let Some(ref db_path) = cli.ccr_db_path {
			if !db_path.as_os_str().is_empty() && !db_path.is_absolute() {
				if let Ok(exe_path) = std::env::current_exe() {
					if let Some(exe_dir) = exe_path.parent() {
						let old = db_path.display().to_string();
						cli.ccr_db_path = Some(exe_dir.join(db_path));
						tracing::info!(
							"resolved relative ccr_db_path from {} to {}",
							old,
							cli.ccr_db_path.as_ref().unwrap().display()
						);
					}
				}
			}
			if let Some(parent) = cli.ccr_db_path.as_ref().and_then(|p| p.parent()) {
				std::fs::create_dir_all(parent)?;
			}
		}
		let listener = tokio::net::TcpListener::bind(cli.listen)
			.await
			.with_context(|| format!("failed to bind listener \"{name}\" on {}", cli.listen))?;
		let state = Arc::new(proxy::build_state(&cli, compression.as_ref()).await?);
		bound.push((name, cli, listener, state));
	}

	// ── Spawn config file watcher for hot-reload ──────────────────
	// Holds a clone of every listener's `Arc<AppState>` so a file change
	// applies to all of them (report 07 F4/T15 - previously this only
	// re-parsed and logged; `AppState` didn't exist yet at this point in
	// startup, so there was nothing live for it to write into).
	let watch_path = {
		let p = std::path::PathBuf::from(&config_path);
		if p.is_relative() {
			std::env::current_dir().unwrap_or_default().join(&p)
		} else {
			p
		}
	};
	let watch_path_str = watch_path.to_string_lossy().to_string();
	let states_for_watcher: Vec<_> = bound.iter().map(|(_, _, _, s)| s.clone()).collect();
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
			},
		};
		let watch_dir = std::path::Path::new(&watch_path).parent().unwrap_or(std::path::Path::new("."));
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
						let thresholds = proxy::resolve_thresholds(config.compression.as_ref());
						for state in &states_for_watcher {
							state.cache_compress_threshold.store(thresholds.cache, Ordering::Relaxed);
							state.token_compress_threshold.store(thresholds.token, Ordering::Relaxed);
							state.inline_ccr_threshold.store(thresholds.inline, Ordering::Relaxed);
							state
								.code_multiplier_x100
								.store((thresholds.code_multiplier * 100.0) as u64, Ordering::Relaxed);
						}
						tracing::info!(
							path = %watch_path_str,
							cache_threshold = thresholds.cache,
							token_threshold = thresholds.token,
							inline_threshold = thresholds.inline,
							code_multiplier = thresholds.code_multiplier,
							listeners = states_for_watcher.len(),
							"config reloaded - compression thresholds applied to all live listeners"
						);
					},
					Err(e) => {
						tracing::warn!(error = %e, "failed to reload config on file change");
					},
				}
			}
		}
	});

	let mut handles = Vec::new();
	for (name, cli, listener, state) in bound {
		let rx = shutdown_rx.clone();
		let handle = tokio::spawn(async move {
			if let Err(e) = run_single(name, cli, listener, state, rx).await {
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

	// Clone abort handles so we can force-kill after handles are moved into
	// join_all
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

	// Wait for graceful drain (via axum's with_graceful_shutdown), second signal,
	// or timeout. NOTE: do NOT re-await drain_fut after select! - it was polled by
	// select! and re-awaiting would be double-poll UB. Abort handles handle the
	// remaining tasks.
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
	cli: Cli,
	listener: tokio::net::TcpListener,
	state: Arc<proxy::AppState>,
	mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
) -> anyhow::Result<()> {
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

	// 02-F1: one-time notice, not per-request - management routes
	// (/stats, /retrieve, /ccr/*, /reload, /tool/relay, ...) accept any
	// loopback caller with no credential until APHRODITE_MGMT_TOKEN is set.
	if mgmt_token().is_none() {
		tracing::warn!(
			name = %name,
			"APHRODITE_MGMT_TOKEN not set - management routes (/stats, /retrieve, /ccr/*, /reload, /tool/relay, ...) accept any loopback caller with no credential"
		);
	}

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
                    // F19: 60s TTL cache - see `upstream_health_cache`'s doc
                    // comment for why an uncached probe here is the same
                    // rate-limit-quota-burning cost class as the health
                    // check this endpoint's sibling `/health` already fixed.
                    const TTL: std::time::Duration = std::time::Duration::from_secs(60);
                    if let Some((ok, at)) = *s.upstream_health_cache.lock().unwrap_or_else(|e| e.into_inner()) {
                        if at.elapsed() < TTL {
                            return Json(serde_json::json!({"upstream": ok, "cached": true})).into_response();
                        }
                    }
                    let ok = s.client
                        .get(format!("{}/models", s.api_url.trim_end_matches('/')))
                        .header("Authorization", format!("Bearer {}", s.api_key.expose()))
                        .timeout(std::time::Duration::from_secs(5))
                        .send()
                        .await
                        .map(|r| r.status().is_success())
                        .unwrap_or(false);
                    if let Ok(mut cache) = s.upstream_health_cache.lock() {
                        *cache = Some((ok, std::time::Instant::now()));
                    }
                    Json(serde_json::json!({"upstream": ok, "cached": false})).into_response()
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
                    // F11: bucket 4 (`record_latency`'s last arm) has no
                    // upper bound - it catches every sample >= 1s, including
                    // 30s+ outliers - so labeling it "10.0" was actively
                    // wrong (Prometheus consumers would assume everything in
                    // it is <= 10s and mis-compute quantiles), and the
                    // missing explicit "+Inf" bucket (required to equal
                    // `_count`) broke `histogram_quantile()`. Relabeling
                    // bucket 4 as "+Inf" is honest about its real bound and
                    // satisfies the convention in one step - no new bucket
                    // needed since it was already unbounded in practice.
                    let le_labels = ["0.001", "0.01", "0.1", "1.0", "+Inf"];
                    for (i, v) in buckets.iter().enumerate() {
                        let le = le_labels.get(i).copied().unwrap_or("+Inf");
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
                    if let Some(c) = ue["connect_errors"].as_u64() { out.push_str(&format!("aphrodite_upstream_connect_errors_total {c}\n")); }
                    // 02-F9: mid-stream SSE chunk errors - added to /stats when
                    // sse_stream_errors was introduced, but missed here.
                    if let Some(c) = ue["sse_stream_errors"].as_u64() { out.push_str(&format!("aphrodite_sse_stream_errors_total {c}\n")); }
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
        // 1 MB body limit on management routes only (F7) - the catch-all
        // proxy route gets its own, much larger limit below: config
        // advertises max_context up to 1,000,000 tokens, and a chat request
        // at even 200k tokens is easily >1 MB of JSON with tool schemas and
        // history, so a blanket 1 MB cap on `/{*path}` rejected large agent
        // conversations with 413 before `proxy_handler` ever ran.
        .layer(DefaultBodyLimit::max(1024 * 1024))
        // 02-F1: bearer-token gate on management routes only - applied
        // before merging with `catch_all` below, so it never guards the
        // actual LLM-proxying `/{*path}` route (a separate concern; that
        // traffic's credential is the upstream API key, not this token).
        .layer(middleware::from_fn(require_mgmt_token));

	// Catch-all proxy route: its own, much larger body limit.
	let catch_all = Router::new()
		.route("/{*path}", any(proxy::proxy_handler))
		.layer(DefaultBodyLimit::max(64 * 1024 * 1024));

	let restricted = restricted
        .merge(catch_all)
        // Loopback enforcement layer on all non-/health routes
        .layer(middleware::from_fn(loopback_only));

	// Public route (no loopback enforcement) merged with restricted routes.
	// report 12 F1: no `CorsLayer` at all - this proxy has no legitimate
	// browser-based consumer (Hermes and the Python plugin are non-browser
	// HTTP clients), so `CorsLayer::permissive()` only ever served to let
	// any website a user visits `fetch()` `/history`/`/retrieve`/`/ccr/*`
	// cross-origin and read or tamper with stored session content -
	// `loopback_only`'s peer-IP check does not stop this, since the
	// browser making the request genuinely is a loopback client.
	let app = Router::new()
		.route("/health", get(health_check))
		.merge(restricted)
		.with_state(state);

	// F9: `listener` was already bound in `run()` before any proxy task was
	// spawned - see the call site for why.
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

/// Hostnames a legitimate loopback caller can present in its `Host` header.
/// report 12 F1: a peer-IP check alone is not DNS-rebinding-safe - a
/// hostile webpage's JS can point a hostname it controls at 127.0.0.1 via
/// DNS, making the TCP connection genuinely loopback while the `Host`
/// header carries the attacker's own domain. Requiring `Host` to name the
/// loopback address itself closes that gap; real clients (Hermes, the
/// Python plugin, `curl`) already address the proxy this way.
const ALLOWED_LOOPBACK_HOSTS: &[&str] = &["localhost", "127.0.0.1", "[::1]", "::1"];

/// Strip an optional trailing `:port` from a `Host` header value - but not
/// from a bracketed IPv6 literal's own colons (`[::1]:9797` -> `[::1]`).
fn host_header_to_hostname(host: &str) -> String {
	if let Some(rest) = host.strip_prefix('[') {
		rest.split(']').next().map(|h| format!("[{h}]")).unwrap_or_default()
	} else {
		host.split(':').next().unwrap_or("").to_string()
	}
}

/// The actual allow/reject decision `loopback_only` enforces, pulled out as
/// a pure function so it's directly unit-testable without constructing a
/// real axum `Request`/`Next` - the middleware below is a thin wrapper that
/// only translates this `Result` into an HTTP response.
fn check_loopback_request(addr: SocketAddr, host_header: &str) -> Result<(), &'static str> {
	if !addr.ip().is_loopback() {
		return Err("only loopback clients allowed");
	}
	// A missing/unparseable Host is rejected, not waved through: peer-IP
	// loopback is the first layer, but this second layer exists specifically
	// to catch a DNS-rebinding request that genuinely originates from
	// loopback yet carries a non-loopback (or absent) Host - HTTP/1.1
	// mandates Host, and every real caller here (curl, the Hermes plugin)
	// sends it, so there's no legitimate case to exempt (02-F6).
	let hostname = host_header_to_hostname(host_header);
	if !ALLOWED_LOOPBACK_HOSTS.contains(&hostname.as_str()) {
		return Err("Host header does not name a loopback address");
	}
	Ok(())
}

/// Middleware that rejects non-loopback clients on all routes except /health.
/// /health is intentionally exempt so external load-balancer probes work.
async fn loopback_only(
	ConnectInfo(addr): ConnectInfo<SocketAddr>,
	request: Request,
	next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
	let host = request
		.headers()
		.get(axum::http::header::HOST)
		.and_then(|v| v.to_str().ok())
		.unwrap_or("");
	if let Err(msg) = check_loopback_request(addr, host) {
		return Err((StatusCode::FORBIDDEN, Json(serde_json::json!({"error": msg}))));
	}
	Ok(next.run(request).await)
}

/// 02-F1: `loopback_only`'s peer-IP + Host check stops a remote peer and a
/// DNS-rebinding *read* - but a hostile local page can still issue a CORS
/// "simple request" write (`fetch(..., {method:"POST", mode:"no-cors"})`
/// never preflights, and the browser genuinely is a loopback client with a
/// valid Host header). A bearer token checked on the management routes
/// (`/stats`, `/history`, `/retrieve`, `/ccr/*`, `/reload`, `/tool/relay`,
/// `/version`, `/stats/db`, `/health/upstream`) closes that: seeding CCR
/// entries, evicting a victim's markers via `/reload`, or reading stored
/// session content via `/retrieve`/`/history` all require it now.
///
/// `/health` is a separate router merged in without this layer at all (load
/// balancer probes need to reach it with no credentials). `/metrics` is
/// exempt by path check below - per 02-F11, a Prometheus scraper typically
/// can't be configured to send a bearer token, and this endpoint predates
/// this layer with an explicit "local-only deployments" comment; `/stats`
/// (which exposes the same data plus request history) is NOT exempt.
///
/// Back-compat default: if `APHRODITE_MGMT_TOKEN` is unset, every request
/// passes (unchanged behavior) - `run_single` logs one startup `warn!` so an
/// operator relying on that default isn't silently unprotected.
fn mgmt_token() -> Option<String> {
	std::env::var("APHRODITE_MGMT_TOKEN").ok().filter(|s| !s.is_empty())
}

/// The actual allow/reject decision, pulled out as a pure function per the
/// same pattern as [`check_loopback_request`] - directly unit-testable.
/// `configured = None` means auth is disabled; always `Ok`.
fn check_bearer_token(configured: Option<&str>, auth_header: &str) -> Result<(), &'static str> {
	let Some(token) = configured else {
		return Ok(());
	};
	if auth_header.strip_prefix("Bearer ") == Some(token) {
		Ok(())
	} else {
		Err("missing or invalid Authorization bearer token")
	}
}

async fn require_mgmt_token(
	request: Request,
	next: Next,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
	if request.uri().path() == "/metrics" {
		return Ok(next.run(request).await);
	}
	let token = mgmt_token();
	let auth = request
		.headers()
		.get(axum::http::header::AUTHORIZATION)
		.and_then(|v| v.to_str().ok())
		.unwrap_or("");
	if let Err(msg) = check_bearer_token(token.as_deref(), auth) {
		return Err((StatusCode::UNAUTHORIZED, Json(serde_json::json!({"error": msg}))));
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

#[cfg(test)]
mod tests {
	use super::*;

	// ── T5 (F1): Host-header validation must accept every loopback form a
	// real client uses and reject a DNS-rebinding attacker's own hostname,
	// which is where the peer-IP-only check (still enforced separately)
	// cannot help - the TCP connection genuinely is loopback in that attack. ──
	#[test]
	fn test_host_header_to_hostname_strips_port() {
		assert_eq!(host_header_to_hostname("127.0.0.1:9797"), "127.0.0.1");
		assert_eq!(host_header_to_hostname("localhost:9798"), "localhost");
	}

	#[test]
	fn test_host_header_to_hostname_no_port() {
		assert_eq!(host_header_to_hostname("127.0.0.1"), "127.0.0.1");
		assert_eq!(host_header_to_hostname("localhost"), "localhost");
	}

	#[test]
	fn test_host_header_to_hostname_ipv6_bracketed() {
		assert_eq!(host_header_to_hostname("[::1]:9797"), "[::1]");
		assert_eq!(host_header_to_hostname("[::1]"), "[::1]");
	}

	#[test]
	fn test_allowed_loopback_hosts_accepts_real_clients() {
		for h in ["127.0.0.1", "localhost", "[::1]"] {
			assert!(ALLOWED_LOOPBACK_HOSTS.contains(&h), "{h} must be an allowed loopback host");
		}
	}

	#[test]
	fn test_allowed_loopback_hosts_rejects_dns_rebinding_hostname() {
		// The exact attack this closes: a hostile page's JS does
		// `fetch("http://attacker.example:9797/retrieve")` where
		// `attacker.example` DNS-resolves to 127.0.0.1 - the TCP peer is
		// loopback (passes the IP check), but the Host header carries the
		// attacker's own domain, not a loopback name.
		let hostname = host_header_to_hostname("attacker.example:9797");
		assert!(!ALLOWED_LOOPBACK_HOSTS.contains(&hostname.as_str()));
	}

	// ── End-to-end checks against `check_loopback_request` - the exact
	// function the middleware calls, not a reimplementation - covering
	// every real caller in this repo plus the attack the change closes. ──

	fn loopback_v4(port: u16) -> SocketAddr {
		use std::net::{Ipv4Addr, SocketAddrV4};
		SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
	}

	fn lan_v4(port: u16) -> SocketAddr {
		use std::net::{Ipv4Addr, SocketAddrV4};
		SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 1, 50), port))
	}

	#[test]
	fn test_check_loopback_request_allows_real_client_host_headers() {
		// The Python plugin's health check and every documented curl/OpenAI-
		// client example in this repo address the proxy as `127.0.0.1:PORT`;
		// a standard HTTP client sets `Host` to exactly that.
		assert!(check_loopback_request(loopback_v4(9797), "127.0.0.1:9797").is_ok());
		assert!(check_loopback_request(loopback_v4(9798), "127.0.0.1:9798").is_ok());
		assert!(check_loopback_request(loopback_v4(9797), "localhost:9797").is_ok());
		assert!(
			check_loopback_request(loopback_v4(9797), "127.0.0.1").is_ok(),
			"no-port Host must also pass"
		);
	}

	// 02-F6: a missing/unparseable Host must be REJECTED, not waved through -
	// otherwise a client that can omit or strip Host (HTTP/1.0, some
	// non-browser `fetch` engines) skips the second defense layer entirely,
	// leaving only the peer-IP check the Host check exists to back up.
	#[test]
	fn test_check_loopback_request_rejects_missing_host_header() {
		assert!(check_loopback_request(loopback_v4(9797), "").is_err());
	}

	#[test]
	fn test_check_loopback_request_allows_ipv6_loopback() {
		use std::net::{Ipv6Addr, SocketAddrV6};
		let addr = SocketAddr::V6(SocketAddrV6::new(Ipv6Addr::LOCALHOST, 9797, 0, 0));
		assert!(check_loopback_request(addr, "[::1]:9797").is_ok());
	}

	#[test]
	fn test_check_loopback_request_rejects_dns_rebinding() {
		// TCP peer genuinely is loopback (this is the scenario DNS
		// rebinding produces - the attacker's hostname resolves to
		// 127.0.0.1), but Host carries the attacker's own domain.
		let result = check_loopback_request(loopback_v4(9797), "attacker.example:9797");
		assert!(result.is_err());
	}

	#[test]
	fn test_check_loopback_request_rejects_non_loopback_peer_regardless_of_host() {
		// Pre-existing behavior must be unchanged: a non-loopback TCP peer
		// is rejected even when it presents a "correct-looking" Host header.
		let result = check_loopback_request(lan_v4(9797), "127.0.0.1:9797");
		assert!(result.is_err());
	}

	#[test]
	fn test_check_loopback_request_rejects_non_loopback_peer_with_no_host() {
		let result = check_loopback_request(lan_v4(9797), "");
		assert!(result.is_err());
	}

	// ── 02-F1: bearer-token gate on management routes. `check_bearer_token`
	// is the pure decision function (no env vars, no axum Request/Next) so
	// these tests don't need env-var serialization like `mgmt_token()`
	// itself would. ──
	#[test]
	fn test_check_bearer_token_passes_when_unconfigured() {
		// Back-compat default: no token configured -> every request passes,
		// regardless of what Authorization header (if any) is present.
		assert!(check_bearer_token(None, "").is_ok());
		assert!(check_bearer_token(None, "Bearer whatever").is_ok());
	}

	#[test]
	fn test_check_bearer_token_accepts_matching_token() {
		assert!(check_bearer_token(Some("secret123"), "Bearer secret123").is_ok());
	}

	#[test]
	fn test_check_bearer_token_rejects_missing_header() {
		assert!(check_bearer_token(Some("secret123"), "").is_err());
	}

	#[test]
	fn test_check_bearer_token_rejects_wrong_token() {
		assert!(check_bearer_token(Some("secret123"), "Bearer wrong").is_err());
	}

	#[test]
	fn test_check_bearer_token_rejects_missing_bearer_prefix() {
		// A raw token with no "Bearer " scheme prefix must not pass, even if
		// the token value itself is correct - the header format matters.
		assert!(check_bearer_token(Some("secret123"), "secret123").is_err());
	}
}
