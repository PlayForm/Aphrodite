//! Build the proxy's `AppState` from CLI configuration.
//!
//! Opens the CCR backend the mode calls for (SQLite for Token, in-memory for
//! Cache), constructs the two HTTP clients (bounded, and stream-client with no
//! total timeout), resolves the thresholds and zeroes every counter. Startup
//! ordering matters here: the stream client exists only because a bounded
//! client's timeout would kill a long SSE response.

use std::{
	collections::{HashMap, VecDeque},
	num::NonZeroUsize,
	sync::{
		Arc,
		Mutex,
		atomic::{AtomicBool, AtomicU64, AtomicUsize},
	},
};

use reqwest::Client as HttpClient;
use headroom_core::ccr::{
	CcrStore,
	backends::{in_memory::InMemoryCcrStore, sqlite::SqliteCcrStore},
};
use tokio_util::task::TaskTracker;

use crate::{
	Configuration::{
		CLI::Struct::Struct as CLI,
		Proxy::{Compression::Struct::Struct as CompressionConfiguration, Mode::Enum::Enum as ProxyMode},
	},
	Proxy::{
		State::Struct::Struct as AppState,
		Threshold::{
			Const::{InitialFillPct, InitialRatioEma},
			Resolve::Fn::Fn as ResolveThresholds,
		},
	},
};

/// Construct `AppState` from CLI config: builds the tuned HTTP client,
/// opens the CCR backend appropriate for `cli.mode` (SQLite for Token,
/// in-memory for Cache), and zeroes all counters.
pub async fn Fn(CLI:&CLI, Compression:Option<&CompressionConfiguration>) -> anyhow::Result<AppState> {
	// Tuned HttpClient for high-concurrency API proxy workload.
	// Default pool: 100 idle connections per host, 90s idle timeout, keepalive.
	let Client = HttpClient::builder()
		.timeout(std::time::Duration::from_secs(CLI.Timeout))
		.connect_timeout(std::time::Duration::from_secs(10))
		.pool_max_idle_per_host(100)
		.pool_idle_timeout(std::time::Duration::from_secs(90))
		.tcp_keepalive(std::time::Duration::from_secs(60))
		.build()?;
	// 02-F2: same tuning, no total timeout - see `AppState::stream_client`'s
	// doc comment.
	let StreamClient = HttpClient::builder()
		.connect_timeout(std::time::Duration::from_secs(10))
		.pool_max_idle_per_host(100)
		.pool_idle_timeout(std::time::Duration::from_secs(90))
		.tcp_keepalive(std::time::Duration::from_secs(60))
		.build()?;

	let ccr:Option<Arc<dyn CcrStore>> = match CLI.Mode {
		ProxyMode::Token if !CLI.NoCCRMarker => {
			let DatabasePath = CLI.CCRDatabasePath.as_ref().map_or_else(
				|| {
					// Runtime home is a single shared decision
					// (Home::Runtime::Fn::Fn: $APHRODITE_HOME -> $HERMES_HOME ->
					// $HOME -> platform default), so the token proxy's SQLite
					// database lives in the same home the plugin shim resolves
					// (issue 40) - never a second, shadow home.
					match crate::Home::Runtime::Opt::Fn::Fn() {
						Some(Home) => Home.join("ccr.db"),
						None => {
							// W2-3: degrade, never bail. A missing home dir
							// (HOME unset or not an absolute path) used to
							// silently fall back to /tmp, which a container
							// restart wipes - the whole SQLite CCR database
							// vanished with it. Warn loudly so the operator
							// knows the DB is ephemeral, and keep going.
							let Fallback = std::path::PathBuf::from("/tmp")
								.join(".hermes")
								.join("aphrodite")
								.join("ccr.db");
							tracing::warn!(
								"home directory unavailable (HOME unset or not absolute) - falling back to {}: the \
								 SQLite CCR database will not survive a container restart",
								Fallback.display()
							);
							Fallback
						},
					}
				},
				|p| p.clone(),
			);
			// Ensure parent directories exist before opening SQLite DB.
			// Without this, a missing ~/.hermes/aphrodite/ directory causes
			// the token proxy to fail silently at startup while the cache
			// proxy continues running - a partial-failure that's invisible
			// to the plugin because stderr is piped to DEVNULL.
			if let Some(Parent) = DatabasePath.parent() {
				std::fs::create_dir_all(Parent)
					.map_err(|e| anyhow::anyhow!("SQLite CCR: cannot create directory {}: {}", Parent.display(), e))?;
			}
			let Store = SqliteCcrStore::open_with_limits(
				&DatabasePath,
				CLI.CCRTtlSeconds,
				Some(CLI.StoreMaximumEntries),
				Some(CLI.StoreMaximumBytes),
			)
			.map_err(|e| anyhow::anyhow!("SQLite CCR: {}", e))?;
			Some(Arc::new(Store))
		},
		ProxyMode::Cache => {
			let Store =
				InMemoryCcrStore::with_capacity_and_ttl(10_000, std::time::Duration::from_secs(CLI.CCRTtlSeconds));
			Some(Arc::new(Store))
		},
		_ => None,
	};

	let Thresholds = ResolveThresholds(Compression);

	Ok(AppState {
		Client,
		StreamClient,
		APIURL:CLI.APIURL.clone(),
		Model:CLI.Model.clone(),
		APIKey:CLI.APIKey.clone().into(),
		ccr,
		AddMarkers:!CLI.NoCCRMarker,
		Mode:CLI.Mode,
		ToolRelay:CLI.ToolRelay,
		NotifyURL:CLI.NotifyURL.clone(),
		NotifyKey:CLI.NotifyKey.clone(),
		Dev:CLI.Dev,
		LatencyBuckets:[
			AtomicU64::new(0),
			AtomicU64::new(0),
			AtomicU64::new(0),
			AtomicU64::new(0),
			AtomicU64::new(0),
		],
		TotalLatencyMicros:AtomicU64::new(0),
		LastErrors:Mutex::new(VecDeque::new()),
		CompressionsByType:Mutex::new(HashMap::new()),
		RequestHistory:Mutex::new(VecDeque::new()),
		InlineCCR:Mutex::new(lru::LruCache::new(NonZeroUsize::new(1024).unwrap())),
		RequestsTotal:AtomicU64::new(0),
		RequestsCompressed:AtomicU64::new(0),
		TokensSaved:AtomicU64::new(0),
		CCRHits:AtomicU64::new(0),
		CCRMisses:AtomicU64::new(0),
		CCRCreated:AtomicU64::new(0),
		ToolRelayCalls:AtomicU64::new(0),
		CompressionRatioEma:AtomicU64::new(InitialRatioEma),
		ResponseCache:Mutex::new(lru::LruCache::new(NonZeroUsize::new(128).unwrap())),
		ResponseCacheTtl:std::time::Duration::from_secs(CLI.CCRTtlSeconds),
		CacheHits:AtomicU64::new(0),
		CacheMisses:AtomicU64::new(0),
		FillPct:AtomicU64::new(InitialFillPct()),
		TaskTracker:TaskTracker::new(),

		InlineCCRHits:AtomicU64::new(0),
		InlineCCRMisses:AtomicU64::new(0),
		ToolRelaySuccess:AtomicU64::new(0),
		ToolRelayFailure:AtomicU64::new(0),
		NotifySuccess:AtomicU64::new(0),
		NotifyFailure:AtomicU64::new(0),
		UpstreamErrors4xx:AtomicU64::new(0),
		UpstreamErrors5xx:AtomicU64::new(0),
		UpstreamTimeouts:AtomicU64::new(0),
		UpstreamConnectErrors:AtomicU64::new(0),
		SSEStreamErrors:AtomicU64::new(0),
		RequestBodyBytes:AtomicU64::new(0),
		ResponseBodyBytes:AtomicU64::new(0),
		UpstreamLatencyMicros:AtomicU64::new(0),
		UpstreamHealthCache:std::sync::Mutex::new(None),
		CacheCompressThreshold:AtomicUsize::new(Thresholds.Cache),
		TokenCompressThreshold:AtomicUsize::new(Thresholds.Token),
		InlineCCRThreshold:AtomicUsize::new(Thresholds.inline),
		CodeMultiplierX100:AtomicU64::new((Thresholds.CodeMultiplier * 100.0) as u64),
		AutoExpand:AtomicBool::new(Thresholds.AutoExpand),
		AutoExpandLimit:AtomicUsize::new(Thresholds.AutoExpandLimit),
		MaximumRequestBodySize:Thresholds.MaximumRequestBodySize,
	})
}

// ── Main proxy handler ──────────────────────────────────────────────
