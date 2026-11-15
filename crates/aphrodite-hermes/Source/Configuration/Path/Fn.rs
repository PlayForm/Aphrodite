//! Predict which compression path a tool call will take.
//!
//! "Path" here is the configured DECISION path, not a filesystem path: the
//! label is derived from the session's config fields (`auto_expand`,
//! `chain_split_enabled`, `live_container`) plus the tool name and its args.
//! It exists only to label the dev-only pre-tool debug line - the transform
//! arms log the ACTUAL outcome - so a wrong prediction mislabels a log line
//! and nothing else.

use aphrodite::State::Struct::AphroditeState;

use crate::Configuration::Path::{Classify, Render};

/// Predict the compression path for a tool call (best-effort label for the
/// `pre_tool` debug line; the transform arms log the actual outcome).
///
/// Classification and rendering are separate leaves on purpose: the decision is
/// a value (`Route::Enum`), and the label is a total function of that value, so
/// a new route cannot reach the log without a match arm naming it.
pub(crate) fn Fn(State:&AphroditeState, Tool:&str, Parsed:&serde_json::Value) -> String {
	Render::Fn::Fn(Classify::Fn::Fn(State, Tool, Parsed)).to_string()
}
