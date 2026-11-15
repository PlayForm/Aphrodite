//! Execute one relayed tool call against this proxy's own state.
//!
//! This is the bridge that lets the Hermes side reach CCR entries the proxy
//! owns (retrieve, search, stats). It is deliberately a closed set of verbs -
//! an unknown tool name is an error, never a fallthrough.

use std::sync::atomic::Ordering;

use anyhow::Context;
use headroom_core::ccr::compute_key;

use crate::{
	Configuration::Proxy::Mode::Enum::Enum as ProxyMode,
	Proxy::{
		Marker::Fn::Fn as SmartMarker,
		State::{
			CCR::{Get::Fn::Fn as CCRGet, Length::Fn::Fn as CCRLength, Put::Fn::Fn as CCRPut},
			Struct::Struct as AppState,
		},
	},
};

/// Dispatch a single tool-relay call by name (`AphroditeRetrieve`,
/// `AphroditeCompress`, `aphrodite_list`) - the actual work behind
/// `POST /tool_relay`, called from [`HandleToolRelay`].
///
/// Every failure is an `anyhow::Error` whose message is the whole diagnosis the
/// caller gets: `/tool_relay` puts `format!("{e:#}")` straight into its `error`
/// field, so an error that does not name the missing parameter or the unknown
/// tool name leaves the client with nothing to act on.
pub(crate) async fn Fn(State:&AppState, Tool:&str, Params:&serde_json::Value) -> anyhow::Result<serde_json::Value> {
	match Tool {
		"aphrodite_retrieve" => {
			let HashRaw = Params
				.get("hash")
				.and_then(|v| v.as_str())
				.context("missing hash parameter for aphrodite_retrieve")?;
			// Strip a `|type|size` marker-body suffix and surrounding
			// whitespace (report 05 F3) - an LLM sometimes echoes the full
			// marker body back as the hash argument instead of the bare
			// hash, and the lookups below (inline_ccr, CCR backend) are
			// exact-match only.
			let hash = crate::Marker::Parse::Normalize::NormalizeHash(HashRaw);
			// Check inline_ccr first (no round-trip needed for tiny entries)
			match State.InlineCCR.lock() {
				Ok(mut Map) => {
					match Map.get(hash) {
						Some(Content) => {
							State.InlineCCRHits.fetch_add(1, Ordering::Relaxed);

							return Ok(serde_json::json!({"found": true, "content": Content.clone()}));
						},
						None => {},
					}
				},
				Err(_) => {},
			}

			State.InlineCCRMisses.fetch_add(1, Ordering::Relaxed);

			// Fallback to CCR store
			match &State.ccr {
				Some(ccr) => {
					match CCRGet(ccr, hash).await {
						Some(Content) => Ok(serde_json::json!({"found": true, "content": Content})),
						None => Ok(serde_json::json!({"found": false})),
					}
				},
				None => anyhow::bail!("CCR not enabled"),
			}
		},
		"aphrodite_compress" => {
			let Content = Params
				.get("content")
				.and_then(|v| v.as_str())
				.context("missing content parameter for aphrodite_compress")?;
			let Center = Params.get("_ccr_center").and_then(|v| v.as_str());
			let hash = compute_key(Content.as_bytes());
			let Size = Content.len();

			match Size < State.InlineCCRThreshold() {
				true => {
					// Tiny content: store inline for the fast path (no CCR backend
					// round-trip), AND to the durable backend when one is
					// configured (report 06 F5) - the inline map is a 1024-entry
					// process-memory LRU, so a busy session can evict this entry
					// within minutes even though the marker handed back looks
					// exactly like a durable one; without the durable copy,
					// `AphroditeRetrieve` after eviction (or a proxy restart)
					// returns `{"found": false}` for a marker the model was told
					// is resolvable. Best-effort: a failed durable put doesn't
					// fail this call since the inline copy still serves reads
					// until it's evicted.
					match State.InlineCCR.lock() {
						Ok(mut Map) => {
							match Map.contains(&hash) {
								true => {
									State.InlineCCRHits.fetch_add(1, Ordering::Relaxed);
								},
								false => {
									State.InlineCCRMisses.fetch_add(1, Ordering::Relaxed);
									Map.put(hash.clone(), Content.to_string());
								},
							}
						},
						Err(_) => {},
					}

					match &State.ccr {
						Some(ccr) => {
							CCRPut(ccr, &hash, Content).await;
						},
						None => {},
					}

					Ok(serde_json::json!({
						"compressed": SmartMarker(&hash, Content, "compress", Center),
						"hash": hash,
						"original_size": Size
					}))
				},
				false => {
					match &State.ccr {
						Some(ccr) => {
							// F4: don't hand back a marker for content that failed to store.
							match !CCRPut(ccr, &hash, Content).await {
								true => {
									anyhow::bail!("failed to store content in CCR backend");
								},
								false => {},
							}

							let Compressed = SmartMarker(&hash, Content, "compress", Center);
							// Savings = bytes removed by the marker replacement, not the
							// bare hash length (report 05 F5).
							State
								.TokensSaved
								.fetch_add(Size.saturating_sub(Compressed.len()) as u64, Ordering::Relaxed);

							Ok(serde_json::json!({
								"compressed": Compressed,
								"hash": hash,
								"original_size": Size
							}))
						},
						None => anyhow::bail!("CCR not enabled"),
					}
				},
			}
		},
		"aphrodite_list" => {
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
		},
		_ => anyhow::bail!("Unknown tool: {}", Tool),
	}
}

// ── Programmatic CCR handlers ────────────────────────────────────────
