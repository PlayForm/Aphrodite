//! Decide, per choice, whether its message content becomes a marker.
//!
//! The pass reads the response and returns one decision per choice; it never
//! mutates the body. That is what lets the accounting be derived from the same
//! list that the applier consumes, and it is why the outcome is a value instead
//! of a `bool` beside a mutated `Value`.
//!
//! `ccr` is required up front: without a store there is nothing to rewrite
//! against, and the old code expressed that as a gate at the call site plus a
//! second `match` inside - two places deciding one thing.
//!
//! Choices are decided sequentially (`StreamExt::then`), not concurrently: the
//! decisions share the store, the counters and the ratio EMA, and running them
//! in parallel would interleave those effects for no gain.

use std::sync::{Arc, atomic::Ordering};

use futures::StreamExt;
use headroom_core::ccr::{CcrStore, compute_key};

use crate::{
	Configuration::Proxy::Mode::Enum::Enum as ProxyMode,
	Proxy::{
		Analysis::{Fn::Fn as AnalyseContent, Struct::Struct as ContentAnalysis},
		Cache::Marker::Fn::Fn as CacheMarker,
		Compress::Chat::{
			Rewrite::Enum::Enum as ContentRewrite,
			Sizing::{Classify::Fn::Fn as ClassifySizing, Enum::Enum as ContentSizing},
		},
		Marker::Fn::Fn as SmartMarker,
		Measurement::Content::Struct::Struct as ContentAccounting,
		Request::Context::Struct::Struct as RequestContext,
		State::Struct::Struct as AppState,
		Storage::{
			Evidence::Struct::Struct as StoreEvidence,
			Store::{Fn::Fn as StoreContent, Outcome::Enum::Enum as StoreOutcome},
		},
	},
};

/// One decision per choice, in choice order.
///
/// `Choices` is the response's `choices` array, handed in by the caller that owns
/// the body: the plan reads it and the applier writes it, and neither has to
/// re-derive where it lives. Taking the array rather than the whole response also
/// stops the plan from being able to mutate what it is deciding about.
pub(crate) async fn Fn(
	Context:&RequestContext<'_>,
	Choices:&[serde_json::Value],
	HeadroomBudget:Option<&str>,
) -> Option<Vec<ContentRewrite>> {
	// No store, nothing to rewrite against. Taken once here and threaded into the
	// decision that writes, so no later stage has to re-ask - the old shape gated
	// at the call site and then decided the same thing again inside `Compress`.
	let ccr = Context.State.ccr.as_ref()?;

	let BaseThreshold = Context.State.CompressThreshold();
	// Headroom budget: lower values compress more aggressively. Coding-tuned:
	// a smooth linear curve from 0.50 (empty) to 1.0 (full), never below 0.5x -
	// semantics and tool chains are worth the tokens.
	let BudgetMult = HeadroomBudget
		.and_then(|Budget| Budget.parse::<f64>().ok())
		.map(|Fill| (0.50 + (Fill / 100.0) * 0.50).clamp(0.50, 1.0))
		.unwrap_or(1.0);

	Some(
		futures::stream::iter(Choices)
			.then(|Choice| Decide(Context, ccr, Choice, BaseThreshold, BudgetMult))
			.collect::<Vec<ContentRewrite>>()
			.await,
	)
}

/// Decide one choice.
async fn Decide(
	Context:&RequestContext<'_>,
	ccr:&Arc<dyn CcrStore>,
	Choice:&serde_json::Value,
	BaseThreshold:usize,
	BudgetMult:f64,
) -> ContentRewrite {
	let Content = match Choice
		.get("message")
		.and_then(|Message| Message.get("content"))
		.and_then(|Slot| Slot.as_str())
	{
		Some(Content) => Content,
		None => return ContentRewrite::Unchanged("not_text"),
	};

	// One analysis, shared by the threshold lookup and the marker renderer, so
	// the type that decides the threshold is the type printed in the marker.
	let Analysis = AnalyseContent(Content);
	let Threshold = (Context.State.ThresholdFor(Analysis.Ct()).max(BaseThreshold) as f64 * BudgetMult) as usize;

	match ClassifySizing(Content.len(), Threshold, Context.State.InlineCCRThreshold()) {
		ContentSizing::Compress => Compress(Context, ccr, &Analysis).await,
		ContentSizing::Inline => Inline(Context, &Analysis),
		ContentSizing::Skip => ContentRewrite::Unchanged("below_threshold"),
	}
}

/// Store the content and replace it with a marker, unless auto-expand asks for
/// the content to be left inline.
async fn Compress(
	Context:&RequestContext<'_>,
	ccr:&Arc<dyn CcrStore>,
	Analysis:&ContentAnalysis<'_>,
) -> ContentRewrite {
	let Content = Analysis.Content();
	let Hash = compute_key(Content.as_bytes());

	// F4: only replace the content with a marker if it is actually retrievable
	// under `hash` - either it was already there (cache hit) or this put
	// succeeded. A failed put must NOT be followed by swapping the response for
	// an unresolvable marker: that would permanently destroy content that never
	// reached the client any other way.
	let Evidence = match StoreContent(ccr, &Hash, Content).await {
		StoreOutcome::Cached(Evidence) => {
			Context.State.CCRHits.fetch_add(1, Ordering::Relaxed);

			Evidence
		},
		StoreOutcome::Stored(Evidence) => {
			Context.State.CCRMisses.fetch_add(1, Ordering::Relaxed);
			Context.State.CCRCreated.fetch_add(1, Ordering::Relaxed);

			Evidence
		},
		StoreOutcome::Failed => {
			Context.State.CCRMisses.fetch_add(1, Ordering::Relaxed);
			tracing::error!(hash = %Hash, "ccr_put failed - leaving content uncompressed to avoid data loss");

			return ContentRewrite::Unchanged("store_failed");
		},
	};

	// Auto-expand: when enabled and the just-stored content fits within the
	// limit, leave the response carrying the original bytes (still stored under
	// `hash`, so later retrievals keep working) and count nothing as compressed,
	// because nothing was removed from the wire.
	let AutoExpand = Context.State.AutoExpand.load(Ordering::Relaxed)
		&& Content.len() <= Context.State.AutoExpandLimit.load(Ordering::Relaxed);

	// The auto-expand decision yields a verdict or nothing; only its absence
	// renders and accounts a marker.
	match AutoExpand.then_some(ContentRewrite::StoredInline) {
		Some(Rewrite) => Rewrite,
		None => {
			let Marker = RenderMarker(Context.State, &Evidence, Analysis);
			// Savings = bytes actually removed from the response (the original
			// content minus the rendered marker that replaces it), not the bare hash
			// length - the marker is hundreds of chars longer than the 40-char hash,
			// so subtracting only the hash overstated savings (report 05 F5).
			let Accounting = ContentAccounting { OriginalBytes:Evidence.Size(), EmittedBytes:Marker.len() };
			Context.State.RecordCompression(Analysis.Ct());
			Context.State.UpdateCompressionRatio(Evidence.Size(), Marker.len());

			ContentRewrite::Replaced { Content:Marker, Accounting }
		},
	}
}

/// Store tiny content inline so a later retrieval needs no backend round-trip.
fn Inline(Context:&RequestContext<'_>, Analysis:&ContentAnalysis<'_>) -> ContentRewrite {
	let Hash = compute_key(Analysis.Content().as_bytes());
	// `put` returns the previous value, which is exactly "was this already
	// here" - one lock, one answer, no read-then-write window. A hit re-stores
	// identical content (the hash is the content's own digest), so the counters
	// keep their meaning without a second lookup.
	let Previous = Context
		.State
		.InlineCCR
		.lock()
		.ok()
		.map(|mut Map| Map.put(Hash, Analysis.Content().to_string()));

	match Previous {
		Some(Some(_)) => {
			Context.State.InlineCCRHits.fetch_add(1, Ordering::Relaxed);
		},
		Some(None) => {
			Context.State.InlineCCRMisses.fetch_add(1, Ordering::Relaxed);
		},
		None => {},
	}

	ContentRewrite::StoredInline
}

/// The marker shape this proxy's mode calls for.
///
/// Rendered from the content the STORE holds, not from the copy that was handed
/// to it: the marker is the model's only handle on that payload, so it describes
/// what a retrieval will actually return.
fn RenderMarker(State:&AppState, Evidence:&StoreEvidence, Analysis:&ContentAnalysis<'_>) -> String {
	// Re-analysed from the bytes the store returned, under the type the detector
	// named: the marker is the model's only handle on that payload, so its
	// preview, metadata and line count all describe what a retrieval will return -
	// not the copy that was handed to the store. `New` re-counts the lines, which
	// is the fact that used to be carried over from the other content.
	let Stored = ContentAnalysis::New(Evidence.Content(), Analysis.Ct());

	match State.Mode {
		ProxyMode::Cache => CacheMarker(Evidence.Hash(), &Stored, None),
		ProxyMode::Token => SmartMarker(Evidence.Hash(), &Stored, None),
	}
}
