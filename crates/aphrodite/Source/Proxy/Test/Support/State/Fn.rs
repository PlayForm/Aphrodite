//! `AppState` fixtures. The 47-field literal lives in one place so a new field
//! touches one test file, not eighty.

use std::{
	collections::VecDeque,
	num::NonZeroUsize,
	sync::atomic::{AtomicBool, AtomicU64, AtomicUsize},
};

use reqwest::Client as HttpClient;
use headroom_core::ccr::backends::in_memory::InMemoryCcrStore;
use tokio_util::task::TaskTracker;

use crate::{
	Configuration::Proxy::Mode::Enum::Enum as ProxyMode,
	Proxy::{
		State::Struct::Struct as AppState,
		Test::Support::Store::Struct::Struct as FailingStore,
		Threshold::Const::{Cache, InitialExpand, InitialFill, InitialRatio, Inline, Token},
	},
};

pub(crate) fn TestState() -> AppState {
	use std::{collections::HashMap, sync::Mutex};
	AppState {
		Client:HttpClient::new(),
		StreamClient:HttpClient::new(),
		APIURL:"https://upstream-openai.com".into(),
		Model:"default-model".into(),
		APIKey:"test".into(),
		ccr:None,
		AddMarkers:false,
		Mode:ProxyMode::Cache,
		ToolRelay:false,
		NotifyURL:None,
		NotifyKey:None,
		Dev:false,
		RequestsTotal:AtomicU64::new(0),
		RequestsCompressed:AtomicU64::new(0),
		TokensSaved:AtomicU64::new(0),
		BytesSaved:AtomicU64::new(0),
		CCRHits:AtomicU64::new(0),
		CCRMisses:AtomicU64::new(0),
		CCRCreated:AtomicU64::new(0),
		RelayCalls:AtomicU64::new(0),
		RatioEma:AtomicU64::new(InitialRatio),
		RequestHistory:Mutex::new(VecDeque::new()),
		InlineCCR:Mutex::new(lru::LruCache::new(NonZeroUsize::new(1024).unwrap())),
		LatencyBuckets:[
			AtomicU64::new(0),
			AtomicU64::new(0),
			AtomicU64::new(0),
			AtomicU64::new(0),
			AtomicU64::new(0),
		],
		TotalLatency:AtomicU64::new(0),
		LastErrors:Mutex::new(VecDeque::new()),
		Compressions:Mutex::new(HashMap::new()),
		ResponseCache:Mutex::new(lru::LruCache::new(NonZeroUsize::new(128).unwrap())),
		CacheTtl:std::time::Duration::from_secs(3600),
		CacheHits:AtomicU64::new(0),
		CacheMisses:AtomicU64::new(0),
		FillPct:AtomicU64::new(InitialFill()),
		TaskTracker:TaskTracker::new(),
		InlineHits:AtomicU64::new(0),
		InlineMisses:AtomicU64::new(0),
		RelaySuccess:AtomicU64::new(0),
		RelayFailure:AtomicU64::new(0),
		NotifySuccess:AtomicU64::new(0),
		NotifyFailure:AtomicU64::new(0),
		UpstreamErrors4xx:AtomicU64::new(0),
		UpstreamErrors5xx:AtomicU64::new(0),
		UpstreamTimeouts:AtomicU64::new(0),
		ConnectErrors:AtomicU64::new(0),
		OtherErrors:AtomicU64::new(0),
		StreamErrors:AtomicU64::new(0),
		RequestBytes:AtomicU64::new(0),
		ResponseBytes:AtomicU64::new(0),
		UpstreamLatency:AtomicU64::new(0),
		UpstreamHealth:std::sync::Mutex::new(None),
		CacheThreshold:AtomicUsize::new(Cache),
		TokenThreshold:AtomicUsize::new(Token),
		InlineThreshold:AtomicUsize::new(Inline),
		CodeMultiplier:AtomicU64::new(300),
		AutoExpand:AtomicBool::new(false),
		ExpandLimit:AtomicUsize::new(InitialExpand),
		BodyLimit:None,
	}
}

/// `test_state()` with a real (in-memory) CCR backend attached, since
/// `Compress` only compresses when `state.ccr` is `Some`.
///
/// `pub(crate)` (rather than private) so other in-crate test modules -
/// e.g. `Retrieve::tests` - can build a real `AppState` without
/// duplicating this ~40-field literal (report 05 T5 verification).
pub(crate) fn WithCCR() -> AppState {
	AppState {
		ccr:Some(std::sync::Arc::new(InMemoryCcrStore::with_capacity_and_ttl(
			1000,
			std::time::Duration::from_secs(300),
		))),
		Mode:ProxyMode::Token,
		..TestState()
	}
}

/// `TestState` with a CCR backend whose writes fail, so the error branches of
/// the store path are exercised without a real SQLite file.
pub(crate) fn FailingCCR() -> AppState {
	AppState {
		ccr:Some(std::sync::Arc::new(FailingStore)),
		Mode:ProxyMode::Token,
		..TestState()
	}
}

// ── Pre-Rust port: `APHRODITE_MAX_REQUEST_BODY_SIZE` - the CappedBody
// extractor must reject over-limit bodies with 413 while reading, and
// `None` must keep today's no-cap behavior byte-identical. ──
/// `TestState` with `[compression] max_request_body_size` set to `n`, so the
/// `CappedBody` extractor has a live cap to enforce. `Some(n)` rejects a body
/// over `n` bytes with 413.
pub(crate) fn WithLimit(n:usize) -> AppState { AppState { BodyLimit:Some(n), ..TestState() } }
