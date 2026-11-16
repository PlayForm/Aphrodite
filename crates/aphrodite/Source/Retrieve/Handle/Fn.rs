//! The `/retrieve` handler: resolve, reassemble, filter, page, report progress.

use std::sync::Arc;

use axum::{
	extract::State,
	response::{IntoResponse, Json},
};

use crate::{
	Proxy::State::Struct::Struct as AppState,
	Retrieve::{
		Filter::Fn::Fn as FilterContent,
		Handle::{Failure, Lookup, Reply},
		Live::{Advance, Lookup::Fn::Fn as LookupLiveContainer},
		Page::{Mode, Render, Slice},
		Request::Struct::Struct as RetrieveRequest,
		Response::Struct::Struct as RetrieveResponse,
		Source,
	},
};

/// Resolve a CCR marker hash back to its original content, optionally filtered by `query` and
/// paged by `offset`/`limit`.
///
/// The handler is a sequence of decisions, and every one of them is a value: where to read from
/// ([`Source::Parse::Fn::Fn`]), what the lookup answered and which store served it
/// ([`Lookup::Fn::Fn`]), what the window asked for ([`Mode::Parse::Fn::Fn`]) and what slicing
/// produced ([`Slice::Fn::Fn`]). It owns the ORDER and nothing else - the two stores, the
/// filter, the window and the progress each live in a leaf, so a change to any one of them does
/// not have to be read out of a 200-line function.
///
/// The order is the contract and does not move: resolve, reassemble a live container, narrow by
/// query, cut the window, then advance the container's progress with the byte count this
/// response actually served.
///
/// The container's reassembled content is TAKEN, not copied: the handler used to clone it out
/// of the lookup's answer and then keep the answer for its metadata, which copied the whole
/// container body on every retrieve of one. Destructuring the two facts costs nothing.
pub async fn Fn(State(state):State<Arc<AppState>>, Json(Request):Json<RetrieveRequest>) -> impl IntoResponse {
	let Source::Enum::Enum::Hash(hash) = Source::Parse::Fn::Fn(Request.hash.as_deref()) else {
		return Reply::Fn::Fn(&Failure::Enum::Enum::MissingHash);
	};

	let (Resolved, Served) = match Lookup::Fn::Fn(&state, &hash).await {
		Ok(Answer) => Answer,
		Err(Failure) => return Reply::Fn::Fn(&Failure),
	};

	// A live container's chunks are its authoritative store, so a container entry is served
	// from its own reassembly and a missing or malformed envelope degrades to the plain entry
	// (warn + degrade, never panic).
	let (Assembled, Meta) = match LookupLiveContainer(&state, &hash, &Resolved) {
		Some((Meta, Assembled)) => (Assembled, Some(Meta)),
		None => (Resolved, None),
	};

	let Mode = Mode::Parse::Fn::Fn(Request.Offset, Request.Limit);
	let Filtered = FilterContent(&Assembled, Request.Query.as_deref());
	let Outcome = match Slice::Fn::Fn(Filtered.as_ref(), &Mode) {
		Ok(Outcome) => Outcome,
		Err(Error) => return Reply::Fn::Fn(&Failure::Enum::Enum::Refused(Error)),
	};

	let Body = Render::Text::Fn::Fn(Filtered.as_ref(), &Outcome);
	let Truncated = Render::Truncated::Fn::Fn(&Outcome);
	let BytesServed = Body.len();

	// Progress is advanced after slicing so the served window's byte count is known, and
	// written back so a later retrieve observes the accumulation.
	let Live = Meta.map(|Meta| {
		let Progress = Advance::Fn::Fn(&Mode, &Meta, BytesServed);

		Advance::Write::Fn::Fn(&state, &hash, &Meta, Progress.BytesArrived);

		Progress
	});

	Json(RetrieveResponse {
		Found:true,
		Content:Some(Body),
		Source:Served.Wire().into(),
		error:None,
		Truncated,
		Live,
	})
	.into_response()
}
