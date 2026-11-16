//! Execute one relayed tool call against this proxy's own state.
//!
//! This is the bridge that lets the Hermes side reach CCR entries the proxy owns
//! (retrieve, compress, list). It is deliberately a closed set of verbs - an
//! unknown tool name is an error, never a fallthrough.
//!
//! Every failure is an `anyhow::Error` whose message is the whole diagnosis the
//! caller gets: `/tool_relay` puts `format!("{e:#}")` straight into its `error`
//! field, so an error that does not name the missing parameter or the unknown
//! tool name leaves the client with nothing to act on.

use std::sync::atomic::Ordering;

use anyhow::Context;
use headroom_core::ccr::compute_key;

use crate::{
	Configuration::Proxy::Mode::Enum::Enum as ProxyMode,
	Proxy::{
		Analysis::Struct::Struct as ContentAnalysis,
		Marker::Fn::Fn as SmartMarker,
		State::{
			CCR::{Get::Fn::Fn as CCRGet, Length::Fn::Fn as CCRLength},
			Struct::Struct as AppState,
		},
		Storage::{
			Put::Fn::Fn as PutContent,
			Target::{Classify::Fn::Fn as ClassifyTarget, Enum::Enum as StorageTarget},
		},
	},
};

/// Dispatch a single tool-relay call by name.
pub(crate) async fn Fn(State:&AppState, Tool:&str, Params:&serde_json::Value) -> anyhow::Result<serde_json::Value> {
	match Tool {
		"aphrodite_retrieve" => Retrieve(State, Params).await,
		"aphrodite_compress" => Compress(State, Params).await,
		"aphrodite_list" => List(State).await,
		_ => anyhow::bail!("Unknown tool: {}", Tool),
	}
}

/// `aphrodite_retrieve` - inline map first, then the durable backend.
async fn Retrieve(State:&AppState, Params:&serde_json::Value) -> anyhow::Result<serde_json::Value> {
	let HashRaw = Params
		.get("hash")
		.and_then(|Value| Value.as_str())
		.context("missing hash parameter for aphrodite_retrieve")?;
	// Strip a `|type|size` marker-body suffix and surrounding whitespace (report
	// 05 F3) - an LLM sometimes echoes the full marker body back as the hash
	// argument instead of the bare hash, and the lookups below are exact-match
	// only.
	let hash = crate::Marker::Parse::Normalize::Fn(HashRaw);

	// The inline map needs no round-trip for tiny entries. One lock, and the hit
	// is a clone out of it - the guard must not outlive this statement.
	let Inline = State.InlineCCR.lock().ok().and_then(|mut Map| Map.get(hash).cloned());
	let Some(Content) = Inline else {
		State.InlineMisses.fetch_add(1, Ordering::Relaxed);

		return match &State.ccr {
			Some(ccr) => {
				match CCRGet(ccr, hash).await {
					Some(Content) => Ok(serde_json::json!({"found": true, "content": Content})),
					None => Ok(serde_json::json!({"found": false})),
				}
			},
			None => anyhow::bail!("CCR not enabled"),
		};
	};

	State.InlineHits.fetch_add(1, Ordering::Relaxed);

	Ok(serde_json::json!({"found": true, "content": Content}))
}

/// `aphrodite_compress` - store the content and hand back a marker.
async fn Compress(State:&AppState, Params:&serde_json::Value) -> anyhow::Result<serde_json::Value> {
	let Content = Params
		.get("content")
		.and_then(|Value| Value.as_str())
		.context("missing content parameter for aphrodite_compress")?;
	let Center = Params.get("_ccr_center").and_then(|Value| Value.as_str());
	let hash = compute_key(Content.as_bytes());
	let Size = Content.len();
	// The caller asked for a marker under a synthetic type name, not a detected
	// one - `compress` is the vocabulary this verb publishes. The line count is
	// still counted from this content, which is what `New` guarantees.
	let Analysis = ContentAnalysis::New(Content, "compress");

	match ClassifyTarget(Size, State.InlineThreshold()) {
		StorageTarget::Inline => {
			// Tiny content: store inline for the fast path, AND to the durable
			// backend when one is configured (report 06 F5) - the inline map is
			// a 1024-entry process-memory LRU, so a busy session can evict this
			// entry within minutes even though the marker handed back looks
			// exactly like a durable one. Best-effort: a failed durable put does
			// not fail this call, since the inline copy still serves reads until
			// it is evicted.
			let _ = State.InlineCCR.lock().map(|mut Map| Map.put(hash.clone(), Content.to_owned()));
			// `OptionFuture` awaits the durable put when there is a backend and
			// does nothing when there is not - a branch-free best-effort, and it
			// cannot leave an unpolled future behind the way an `Option::map`
			// whose result is dropped would.
			futures::future::OptionFuture::from(State.ccr.as_ref().map(|ccr| PutContent(ccr, &hash, Content))).await;

			Ok(serde_json::json!({
				"compressed": SmartMarker(&hash, &Analysis, Center),
				"hash": hash,
				"original_size": Size
			}))
		},
		StorageTarget::Durable => {
			let ccr = State.ccr.as_ref().context("CCR not enabled")?;
			let Evidence = PutContent(ccr, &hash, Content)
				.await
				.context("failed to store content in CCR backend")?;
			let Compressed = SmartMarker(Evidence.Hash(), &Analysis, Center);
			// Savings = bytes removed by the marker replacement, not the bare hash
			// length (report 05 F5).
			State
				.TokensSaved
				.fetch_add(Size.saturating_sub(Compressed.len()) as u64, Ordering::Relaxed);

			Ok(serde_json::json!({
				"compressed": Compressed,
				"hash": hash,
				"original_size": Size
			}))
		},
	}
}

/// `aphrodite_list` - entry count and backend kind.
async fn List(State:&AppState) -> anyhow::Result<serde_json::Value> {
	let Entries = match &State.ccr {
		Some(ccr) => CCRLength(ccr).await,
		None => 0,
	};

	Ok(serde_json::json!({
		"entries": Entries,
		"backend": match State.Mode {
			ProxyMode::Cache => "in_memory",
			ProxyMode::Token => "sqlite",
		},
	}))
}
