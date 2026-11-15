//! The status surface's two contracts: byte-identical output when
//! `LiveContainer` is off, and the arrival/ETA fields added alongside the
//! base fields when it is on.

use crate::{
	Prefetch::{Files::Fn::PrefetchFiles, Status::Fn::PrefetchStatus},
	State::Struct::AphroditeState,
};

// ── Live containers (pre-Rust `_hooks/live.py` port): prefetch status
// surfaces container semantics (arrival timestamp, byte progress, ETA)
// when `state.live_container` is on, and is byte-identical to the
// pre-port renderer when off. ──

#[test]
fn TestPrefetchStatusLiveOffByteIdentical() {
	let mut s = AphroditeState::default();
	assert!(!s.LiveContainer, "default state must have live_container off");
	let Src = env!("CARGO_MANIFEST_DIR").to_string() + "/Source/Prefetch/mod.rs";
	let r = PrefetchFiles(&mut s, std::slice::from_ref(&Src));
	assert_eq!(r["loaded"], 1, "prefetch failed: {r:?}");

	// Reference renderer: the exact algorithm the pre-port bridge
	// handler (aphrodite-hermes `Source/Tool/mod.rs`
	// `aphrodite_prefetch_status`) used - ready entries are
	// `{path, hash, type, size}` from marker meta, plus empty
	// loading/errors arrays and total_ready.
	let ExpectedReady:Vec<serde_json::Value> =
		s.RecentMarkers
			.iter()
			.filter_map(|Mk| {
				Mk.meta.as_ref().and_then(|meta| meta.get("path")).map(
					|path| serde_json::json!({"path": path, "hash": &Mk.hash, "type": Mk.CCRType, "size": Mk.Size}),
				)
			})
			.collect();
	let Expected = serde_json::json!({
		"loading": [],
		"ready": ExpectedReady,
		"errors": [],
		"total_ready": ExpectedReady.len(),
	});

	let Status = PrefetchStatus(&s);
	assert_eq!(
		Status, Expected,
		"live_container off must be byte-identical to the pre-port renderer"
	);
	// No live-container keys anywhere on the surface.
	assert!(Status.get("live_container").is_none());
	for Entry in Status["ready"].as_array().unwrap() {
		assert!(Entry.get("live").is_none());
		assert!(Entry.get("arrived_at").is_none());
		assert!(Entry.get("bytes_arrived").is_none());
		assert!(Entry.get("eta_s").is_none());
	}
}

#[test]
fn TestPrefetchStatusLiveOnSurfacesProgressEta() {
	let mut s = AphroditeState::default();
	s.LiveContainer = true;
	let Src = env!("CARGO_MANIFEST_DIR").to_string() + "/Source/Prefetch/mod.rs";
	let r = PrefetchFiles(&mut s, std::slice::from_ref(&Src));
	assert_eq!(r["loaded"], 1, "prefetch failed: {r:?}");

	let Status = PrefetchStatus(&s);
	assert_eq!(Status["live_container"], true);
	let Ready = Status["ready"].as_array().unwrap();
	assert_eq!(Ready.len(), 1, "one loaded entry: {Status}");
	let Entry = &Ready[0];
	// Base fields must survive intact.
	assert!(Entry.get("path").is_some());
	assert!(Entry.get("hash").is_some());
	assert!(Entry.get("type").is_some());
	// Container semantics: arrival timestamp, byte progress, ETA.
	assert_eq!(Entry["live"], true);
	assert_eq!(Entry["bytes_arrived"], Entry["size"], "sync prefetch arrives fully");
	assert_eq!(Entry["eta_s"], 0.0, "fully arrived -> zero ETA");
	assert!(Entry["arrived_at"].as_u64().unwrap() > 0, "arrival timestamp must be set");
	// The status surface carries the same full 40-char hash the
	// prefetch response returned (instant-marker contract).
	assert_eq!(Entry["hash"].as_str().unwrap().len(), 40);
	assert_eq!(Entry["hash"].as_str().unwrap(), r["results"][0]["hash"].as_str().unwrap());
}

#[test]
fn TestPrefetchInstantMarkerContract() {
	let mut s = AphroditeState::default();
	s.LiveContainer = true;
	let Src = env!("CARGO_MANIFEST_DIR").to_string() + "/Source/Prefetch/mod.rs";
	let FileText = std::fs::read_to_string(&Src).unwrap();
	let r = PrefetchFiles(&mut s, std::slice::from_ref(&Src));
	assert_eq!(r["loaded"], 1);

	// Instant marker: the task hash is available immediately in the
	// prefetch response and on the status surface.
	let hash = r["results"][0]["hash"].as_str().unwrap().to_string();
	assert_eq!(hash.len(), 40, "instant hash must be the full 40-char CCR hash");
	assert_eq!(PrefetchStatus(&s)["ready"][0]["hash"].as_str().unwrap(), hash);

	// Full content arrives later: stored in the inline store and
	// retrievable by the instant hash.
	let Stored = s.InlineStoreGet(&hash);
	assert_eq!(
		Stored.as_deref(),
		Some(FileText.as_str()),
		"full content must be retrievable via the instant hash"
	);
}
