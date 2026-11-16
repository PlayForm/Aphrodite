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
		Test::Support::Store::Struct::Struct as FailingCCRStore,
		Threshold::Const::{
			AutoExpandLimitDefault,
			CacheCompressThreshold,
			InitialFillPct,
			InitialRatioEma,
			InlineCCRThreshold,
			TokenCompressThreshold,
		},
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
		BodyBytesSaved:AtomicU64::new(0),
		CCRHits:AtomicU64::new(0),
		CCRMisses:AtomicU64::new(0),
		CCRCreated:AtomicU64::new(0),
		ToolRelayCalls:AtomicU64::new(0),
		CompressionRatioEma:AtomicU64::new(InitialRatioEma),
		RequestHistory:Mutex::new(VecDeque::new()),
		InlineCCR:Mutex::new(lru::LruCache::new(NonZeroUsize::new(1024).unwrap())),
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
		ResponseCache:Mutex::new(lru::LruCache::new(NonZeroUsize::new(128).unwrap())),
		ResponseCacheTtl:std::time::Duration::from_secs(3600),
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
		UpstreamOtherErrors:AtomicU64::new(0),
		SSEStreamErrors:AtomicU64::new(0),
		RequestBodyBytes:AtomicU64::new(0),
		ResponseBodyBytes:AtomicU64::new(0),
		UpstreamLatencyMicros:AtomicU64::new(0),
		UpstreamHealthCache:std::sync::Mutex::new(None),
		CacheCompressThreshold:AtomicUsize::new(CacheCompressThreshold),
		TokenCompressThreshold:AtomicUsize::new(TokenCompressThreshold),
		InlineCCRThreshold:AtomicUsize::new(InlineCCRThreshold),
		CodeMultiplierX100:AtomicU64::new(300),
		AutoExpand:AtomicBool::new(false),
		AutoExpandLimit:AtomicUsize::new(AutoExpandLimitDefault),
		MaximumRequestBodySize:None,
	}
}

/// `test_state()` with a real (in-memory) CCR backend attached, since
/// `CompressChatCompletion` only compresses when `state.ccr` is `Some`.
///
/// `pub(crate)` (rather than private) so other in-crate test modules -
/// e.g. `Retrieve::tests` - can build a real `AppState` without
/// duplicating this ~40-field literal (report 05 T5 verification).
pub(crate) fn TestStateWithCCR() -> AppState {
	AppState {
		ccr:Some(std::sync::Arc::new(InMemoryCcrStore::with_capacity_and_ttl(
			1000,
			std::time::Duration::from_secs(300),
		))),
		Mode:ProxyMode::Token,
		..TestState()
	}
}

pub(crate) fn TestStateWithFailingCCR() -> AppState {
	AppState {
		ccr:Some(std::sync::Arc::new(FailingCCRStore)),
		Mode:ProxyMode::Token,
		..TestState()
	}
}

// ── Pre-Rust port: `APHRODITE_MAX_REQUEST_BODY_SIZE` - the CappedBody
// extractor must reject over-limit bodies with 413 while reading, and
// `None` must keep today's no-cap behavior byte-identical. ──
pub(crate) fn TestStateWithMaximumBody(n:usize) -> AppState {
	AppState { MaximumRequestBodySize:Some(n), ..TestState() }
}
