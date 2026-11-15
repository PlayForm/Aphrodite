//! Live containers: chunk arithmetic, store layout, instant marker.

use crate::Hook::Live::Chunk::Fn::Fn as ChunkOnCharBoundaries;
use crate::Hook::Live::Const::LiveChunkSize;
use crate::Hook::Live::Key::Fn::LiveChunkKey;
use crate::Hook::Live::Key::Fn::LiveMetaKey;
use crate::Hook::Terminal::Entry::Fn::TransformTerminalOutput;
use crate::Hook::ToolResult::Entry::Fn::TransformToolResult;
use crate::State::Struct::AphroditeState;

// Contract 1: `LiveContainer` off (the default) must keep the payload
// byte-identical to the pre-live shape - same field set, same marker
// grammar, no container fields, no container store keys.
#[test]
fn TestLiveContainerOffPayloadIsByteIdentical() {
	let mut s = AphroditeState::default(); // live_container = false
	s.ToolThreshold = 0;
	let Content = "fn main() {\n    println!(\"hello\");\n}\n";
	let r = TransformToolResult(&mut s, Content, "terminal");
	assert_eq!(r["status"], "ok");
	assert_eq!(r["compressed"], true);
	assert!(r.get("live").is_none(), "off must not emit container fields");
	let hash = r["hash"].as_str().unwrap();
	let ExpectMarker = format!(
		"<<<CCR:{}|{}|{}>>>\n{}",
		hash,
		r["type"].as_str().unwrap(),
		Content.len(),
		r["preview"].as_str().unwrap()
	);
	assert_eq!(r["marker"], ExpectMarker, "off must keep the exact marker grammar");
	assert!(
		!s.InlineStore.contains_key(&LiveMetaKey(hash)),
		"off must not store a container envelope"
	);
	assert!(
		!s.InlineStore.contains_key(&LiveChunkKey(hash, 0)),
		"off must not store container chunks"
	);
	assert_eq!(crate::Resolve::Expand::Fn(&mut s, hash), Some(Content.to_string()));
}

// Contract 2: enabled but below the compress threshold → unchanged
// (no container) on BOTH the tool and terminal paths.
#[test]
fn TestLiveContainerBelowThresholdUnchanged() {
	let mut s = AphroditeState::default();
	s.LiveContainer = true;
	s.ToolThreshold = 10_000;
	let r = TransformToolResult(&mut s, "short output", "terminal");
	assert_eq!(r["compressed"], false);
	assert_eq!(r["reason"], "below_threshold");
	assert!(r.get("live").is_none(), "below-threshold must not emit a container");

	let mut S2 = AphroditeState::default();
	S2.LiveContainer = true;
	S2.TerminalThreshold = 10_000;
	let R2 = TransformTerminalOutput(&mut S2, "tiny terminal output");
	assert_eq!(R2["compressed"], false);
	assert_eq!(R2["reason"], "below_threshold");
	assert!(R2.get("live").is_none());
}

// Contract 3: enabled + over the threshold → INSTANT marker with a
// progress annotation; the store carries the container (meta envelope +
// per-chunk entries) AND the round-trip hash; the marker's hash still
// resolves to the exact original.
#[test]
fn TestLiveContainerToolOverThresholdInstantMarkerWithProgress() {
	let mut s = AphroditeState::default();
	s.LiveContainer = true;
	s.ToolThreshold = 0; // always compress
	let Content = (0..40)
		.map(|i| format!("tool payload line {i:02} with some text\n"))
		.collect::<String>();
	let Chunks = ChunkOnCharBoundaries(&Content, LiveChunkSize);
	assert!(Chunks.len() >= 2, "test content must span multiple chunks");
	let r = TransformToolResult(&mut s, &Content, "terminal");
	assert_eq!(r["status"], "ok");
	assert_eq!(r["compressed"], true);
	assert_eq!(r["live"], true);
	assert_eq!(r["type"], "live", "the marker type must be `live`");
	assert_eq!(r["size"].as_u64().unwrap() as usize, Content.len());
	assert_eq!(r["chunks"].as_u64().unwrap() as usize, Chunks.len());
	assert_eq!(r["bytes_arrived"], 0, "the instant marker reports zero bytes arrived");
	let hash = r["hash"].as_str().unwrap();
	let Marker = r["marker"].as_str().unwrap();
	assert!(
		Marker.starts_with(&format!("<<<CCR:{}|live|{}>>>", hash, Content.len())),
		"marker must use the live grammar: {Marker}"
	);
	assert!(Marker.contains("Live container"), "marker must carry the annotation: {Marker}");
	assert!(
		Marker.contains(&format!("{} chunks", Chunks.len())),
		"marker must carry the chunk count: {Marker}"
	);
	assert!(
		Marker.contains("0 bytes arrived"),
		"marker must carry the progress annotation: {Marker}"
	);
	// Container store: meta envelope + per-chunk entries + round-trip hash.
	assert!(s.InlineStore.contains_key(&LiveMetaKey(hash)), "meta envelope must be stored");
	for i in 0..Chunks.len() {
		assert!(s.InlineStore.contains_key(&LiveChunkKey(hash, i)), "chunk {i} must be stored");
	}
	assert_eq!(crate::Resolve::Expand::Fn(&mut s, hash), Some(Content.clone()));
	assert!(
		s.RecentMarkers.iter().any(|m| m.hash == hash && m.CCRType == "live"),
		"the live entry must be recorded in the catalog"
	);
}

// Contract 3 (terminal path): same instant-marker contract through
// `TransformTerminalOutput`.
#[test]
fn TestLiveContainerTerminalOverThresholdInstantMarker() {
	let mut s = AphroditeState::default();
	s.LiveContainer = true;
	s.TerminalThreshold = 0;
	let Content = (0..40)
		.map(|i| format!("cargo step {i:02} done - line content\n"))
		.collect::<String>();
	let r = TransformTerminalOutput(&mut s, &Content);
	assert_eq!(r["live"], true);
	assert_eq!(r["type"], "live");
	assert_eq!(
		r["chunks"].as_u64().unwrap() as usize,
		ChunkOnCharBoundaries(&Content, LiveChunkSize).len()
	);
	let hash = r["hash"].as_str().unwrap();
	assert!(
		r["marker"]
			.as_str()
			.unwrap()
			.starts_with(&format!("<<<CCR:{}|live|{}>>>", hash, Content.len()))
	);
	assert_eq!(crate::Resolve::Expand::Fn(&mut s, hash), Some(Content));
}

// Contract 4: the default state keeps `LiveContainer` off - no config
// change, no behavioral drift.
#[test]
fn TestLiveContainerDefaultStateUnchanged() {
	let s = AphroditeState::default();
	assert_eq!(s.LiveContainer, false, "default state must keep live_container off");
}

// Auto-expand wins over the container path: under auto-expand (within
// the limit) the hook returns the expanded content inline even when
// live_container is on - no container marker may leak.
#[test]
fn TestLiveContainerDefersToAutoExpandWithinLimit() {
	let mut s = AphroditeState::default();
	s.LiveContainer = true;
	s.AutoExpand = true;
	s.AutoExpandLimit = 102400;
	s.ToolThreshold = 0;
	let Content = "some content that would otherwise become a live container\n";
	let r = TransformToolResult(&mut s, Content, "terminal");
	assert_eq!(r["auto_expanded"], true, "auto-expand must win over the container path");
	assert!(r.get("live").is_none());
	assert_eq!(r["expanded"], Content);
}
