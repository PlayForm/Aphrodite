//! Compress the tool output inside a Chat Completions response.
//!
//! Plan, then apply, then hand back the outcome: the response body and the
//! decisions travel together so the responder can decide the headers, the
//! counters and the accounting from the same list that produced the bytes.
//!
//! `None` means nothing was attempted - the body is not a completion envelope,
//! or no store is configured. A body that was examined and left alone comes back
//! as `Some` with every decision `Unchanged`, which is a different fact and is
//! what `Outcome::Compressed` distinguishes.

use crate::Proxy::{
	Compress::Chat::{
		Apply::Fn::Fn as ApplyRewrites,
		Outcome::Struct::Struct as CompressOutcome,
		Plan::Fn::Fn as PlanRewrites,
	},
	Log::Dev::Fn::Fn as DevLog,
	Request::Context::Struct::Struct as RequestContext,
};

/// Rewrite the completion's oversized tool output.
pub(crate) async fn Fn(
	Context:&RequestContext<'_>,
	ResponseBody:&[u8],
	HeadroomBudget:Option<&str>,
) -> Option<CompressOutcome> {
	let mut Response:serde_json::Value = serde_json::from_slice(ResponseBody).ok()?;
	let Rewrites = PlanRewrites(Context, &Response, HeadroomBudget).await?;

	ApplyRewrites(&mut Response, &Rewrites);

	let Outcome = CompressOutcome { Response, Rewrites };
	let Rewritten = Outcome.Rewritten();

	DevLog(Context.State.Dev, || {
		let Summary = Outcome
			.Rewrites
			.iter()
			.map(|Rewrite| Rewrite.Summary())
			.collect::<Vec<_>>()
			.join(",");

		tracing::info!(ID = %Context.ID, Rewrites = %Summary, "<<< REWRITE")
	});

	// `None` when nothing was stored at all: a below-threshold body, a disabled
	// store, or a failed write. `Some` covers both a marker replacement and an
	// inline store, because the two differ on the wire - which
	// `Outcome::Compressed` answers - and the caller needs to see both.
	Rewritten.then_some(Outcome)
}
