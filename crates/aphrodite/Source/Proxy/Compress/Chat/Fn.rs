//! Compress the tool output inside a Chat Completions response.
//!
//! Walks the completion's messages, replaces oversized tool results with CCR
//! markers and returns the rewritten body. It does *not* inject the
//! `AphroditeRetrieve` tool definition into `tool_calls` - that was tried and
//! reverted (Bug 18); the Python plugin registers the tool instead. Injecting it
//! here makes the upstream see a tool it never advertised.

use std::sync::atomic::Ordering;

use headroom_core::ccr::compute_key;

use crate::{
	Configuration::Proxy::Mode::Enum::Enum as ProxyMode,
	Proxy::{
		Cache::Marker::Fn::Fn as CacheMarker,
		Detect::Fn::Fn as ProxyDetectContentType,
		Marker::Fn::Fn as SmartMarker,
		State::{
			CCR::{Get::Fn::Fn as CCRGet, Put::Fn::Fn as CCRPut},
			Struct::Struct as AppState,
		},
	},
};

/// Compress a Chat Completions API response with smart markers.
pub(crate) async fn Fn(State:&AppState, ResponseBody:&[u8], HeadroomBudget:Option<&str>) -> Option<serde_json::Value> {
	let mut response:serde_json::Value = serde_json::from_slice(ResponseBody).ok()?;
	let Choices = response.get_mut("choices")?.as_array_mut()?;
	let BaseThreshold = State.CompressThreshold(); // floor threshold for all types

	// Headroom budget: lower values compress more aggressively.
	// Coding-tuned: smooth linear curve from 0.50 (empty) to 1.0 (full).
	// Never below 0.5× - semantics and tool chains are worth the tokens.
	let BudgetMult = HeadroomBudget
		.and_then(|b| {
			let Value:f64 = b.parse().ok()?;
			// Linear interpolation: 0.50 + (fill% * 0.50), clamped [0.50, 1.0]
			Some((0.50 + (Value / 100.0) * 0.50).clamp(0.50, 1.0))
		})
		.unwrap_or(1.0);
	let mut DidCompress = false;

	for Choice in Choices {
		let Message = Choice.get_mut("message")?;

		// Compress text content with smart markers
		if let Some(ContentValue) = Message.get_mut("content")
			&& let Some(Content) = ContentValue.as_str()
		{
			let Ct = ProxyDetectContentType(Content);
			let Threshold = (State.ThresholdFor(Ct).max(BaseThreshold) as f64 * BudgetMult) as usize;
			if Content.len() > Threshold {
				if let Some(ccr) = &State.ccr {
					let hash = compute_key(Content.as_bytes());
					// F4: only replace `Content` with a marker if the content is
					// actually retrievable under `hash` - either it was already
					// there (cache hit) or this `Put` succeeded. A failed put
					// (store full/locked/panicked) must NOT be followed by
					// swapping the response for an unresolvable marker - that
					// would permanently destroy content that never reached the
					// client any other way.
					let Stored = if CCRGet(ccr, &hash).await.is_some() {
						State.CCRHits.fetch_add(1, Ordering::Relaxed);
						true
					} else {
						State.CCRMisses.fetch_add(1, Ordering::Relaxed);
						let Okay = CCRPut(ccr, &hash, Content).await;
						if Okay {
							State.CCRCreated.fetch_add(1, Ordering::Relaxed);
						} else {
							tracing::error!(hash = %hash, "ccr_put failed - leaving content uncompressed to avoid data loss");
						}
						Okay
					};
					if Stored {
						// Auto-expand (pre-Rust port): when enabled and the
						// just-stored content fits within `AutoExpandLimit`,
						// return it INLINE instead of emitting a marker - the
						// response carries the original bytes (still stored
						// under `hash`, so later retrievals keep working), and
						// nothing is counted as compressed because nothing was
						// removed from the wire. Over-limit content falls
						// through to the marker path below. Default (flag
						// false) is byte-identical current behavior.
						if State.AutoExpand.load(Ordering::Relaxed)
							&& Content.len() <= State.AutoExpandLimit.load(Ordering::Relaxed)
						{
							*ContentValue = serde_json::Value::String(Content.to_string());
							continue;
						}
						let (Compressed, OrigLength) = {
							let Compressed = match State.Mode {
								ProxyMode::Cache => CacheMarker(&hash, Content, Ct, None),
								ProxyMode::Token => SmartMarker(&hash, Content, Ct, None),
							};
							let len = Content.len();
							State.RecordCompression(Ct);
							(Compressed, len)
						};
						let MarkerLength = Compressed.len();
						// Savings = bytes actually removed from the response
						// (original content minus the rendered marker that
						// replaces it), not the bare hash length - the marker
						// is hundreds of chars longer than the 40-char hash,
						// so subtracting only `hash.len()` overstated savings
						// (report 05 F5). Unit is bytes throughout - see
						// `TokensSaved`'s field doc for the naming caveat.
						State
							.TokensSaved
							.fetch_add(OrigLength.saturating_sub(MarkerLength) as u64, Ordering::Relaxed);
						*ContentValue = serde_json::Value::String(Compressed);
						DidCompress = true;
						State.UpdateCompressionRatio(OrigLength, MarkerLength);
					}
				}
			} else if Content.len() > State.InlineCCRThreshold() {
				// Below compression threshold but above inline threshold: store in inline_ccr
				// so later retrievals can find tiny entries without a backend round-trip.
				let hash = compute_key(Content.as_bytes());
				if let Ok(mut Map) = State.InlineCCR.lock() {
					if Map.contains(&hash) {
						State.InlineCCRHits.fetch_add(1, Ordering::Relaxed);
					} else {
						State.InlineCCRMisses.fetch_add(1, Ordering::Relaxed);
						Map.put(hash, Content.to_string());
					}
				}
			}
		}

		// 02-F3: `tool_calls[].function.arguments` is JSON the CLIENT feeds
		// straight into its own tool executor - it is not model-facing prose
		// the model could choose to `AphroditeRetrieve`. A prior version of
		// this function compressed it into a CCR marker like any other
		// content, which meant the client tried to execute a tool call with
		// `arguments = "<<<CCR:...>>>"` - unparseable JSON that breaks every
		// real OpenAI-tools client. Tool call arguments now always pass
		// through untouched; only `message.content` (model-facing text,
		// handled above) is a legitimate compression target here.
	}

	if DidCompress { Some(response) } else { None }
}

// ── Tool relay handler ───────────────────────────────────────────────
