//! Compress the tool output inside a Chat Completions response.
//!
//! Plan, then apply, then hand back the outcome: the response body and what each
//! choice became travel together, so the responder can decide the headers, the
//! counters and the accounting from the same pass that produced the bytes.
//!
//! `None` means nothing was attempted - the body is not a completion envelope,
//! or no store is configured. A body that was examined and left alone comes back
//! as `Some` with every summary `Unchanged`, which is a different fact and is
//! what `Outcome::Compressed` distinguishes.

use crate::Proxy::{
	Compress::Chat::{
		Apply::Fn::Fn as ApplyRewrites,
		Outcome::{Struct::Struct as CompressOutcome, Summary::Enum::Enum as RewriteSummary},
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
	// The choices array is taken once and handed to both stages - the plan reads
	// it, the apply writes it - so neither has to re-derive where it lives, and the
	// plan cannot mutate what it is deciding about.
	let Choices = Response.get_mut("choices")?.as_array_mut()?;
	let Rewrites = PlanRewrites(Context, Choices.as_slice(), HeadroomBudget).await?;
	let Summaries = ApplyRewrites(Choices.as_mut_slice(), Rewrites);

	let Outcome = CompressOutcome { Response, Summaries };

	DevLog(Context.State.Dev, || {
		let Summary = Outcome
			.Summaries
			.iter()
			.map(RewriteSummary::Summary)
			.collect::<Vec<_>>()
			.join(",");

		tracing::info!(ID = %Context.ID, Rewrites = %Summary, "<<< REWRITE")
	});

	// `None` when nothing was stored at all: a below-threshold body, a disabled
	// store, or a failed write. `Some` covers both a marker replacement and an
	// inline store, because the two differ on the wire - which
	// `Outcome::Compressed` answers - and the caller needs to see both.
	Outcome.Rewritten().then_some(Outcome)
}
