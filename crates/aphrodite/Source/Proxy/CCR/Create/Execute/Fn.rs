//! Store the content, then count it.
//!
//! F14/F4: unavailability and a failed write are rejections, not a fabricated
//! success - the endpoint used to answer with a hash and a savings ratio even
//! when nothing was stored, so every later `/retrieve` of that hash 404s.
//!
//! The counters are incremented only after the evidence exists, which is what
//! makes `ccr_created` and `tokens_saved` describe real entries.

use std::sync::atomic::Ordering;

use crate::Proxy::{
	CCR::Create::{
		Command::Struct::Struct as CreateCommand,
		Outcome::Struct::Struct as CreateOutcome,
		Rejection::Enum::Enum as CreateRejection,
	},
	State::Struct::Struct as AppState,
	Storage::Put::Fn::Fn as PutContent,
};

/// Store the command's content.
pub(crate) async fn Fn(State:&AppState, Command:CreateCommand) -> Result<CreateOutcome, CreateRejection> {
	let ccr = State.ccr.as_ref().ok_or(CreateRejection::NotEnabled)?;
	let Evidence = PutContent(ccr, &Command.Hash, &Command.Content)
		.await
		.ok_or(CreateRejection::StoreFailed)?;

	State.CCRCreated.fetch_add(1, Ordering::Relaxed);
	State.RequestsCompressed.fetch_add(1, Ordering::Relaxed);
	// The wire contract is the bare hash, so the hash's length is the right
	// subtractee here.
	State.TokensSaved.fetch_add(
		Command.Content.len().saturating_sub(Command.Hash.len()) as u64,
		Ordering::Relaxed,
	);

	let CompressedSize = Command.Hash.len();

	Ok(CreateOutcome {
		Hash:Evidence.Hash().to_owned(),
		OriginalSize:Command.Content.len(),
		CompressedSize,
		Estimate:Command.Estimate,
		Notification:Command.Notification,
	})
}
