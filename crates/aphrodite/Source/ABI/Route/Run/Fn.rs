//! `Run`: a prepared command's JSON envelope, under the session lock.

use crate::{
	ABI::Route::{Command::Enum::Enum as Command, Prepared::Enum::Enum as Prepared},
	State::Struct::AphroditeState,
};

/// Run a prepared command and answer with its envelope.
///
/// Infallible: every arm produces a `serde_json::Value`, and a failure a caller
/// should see is already an `{"error": ...}` envelope produced by the command
/// itself. That is why the return type is not a `Result` - a `Result` here would
/// have to invent an error variant for "the command said no", which is not the
/// same thing as "the call failed".
///
/// The `Unknown` arm is a value, not a fallthrough: a hook name the engine does
/// not serve is decoded to `Command::Unknown` and answered with
/// `{"error":"unknown hook: <name>"}`, so a host that misspells a hook learns it
/// immediately instead of receiving an empty result.
pub(crate) fn Fn(State:&mut AphroditeState, Prepared:Prepared<'_>) -> serde_json::Value {
	match Prepared {
		Prepared::Prefetch { Outcomes } => crate::Prefetch::Insert::Fn::InsertOutcomes(State, Outcomes),
		Prepared::Ready(Command) => {
			match Command {
				Command::SessionStart => crate::Hook::Session::Fn::Fn(State),
				Command::TransformToolResult { Content, Tool } => {
					crate::Hook::ToolResult::Entry::Fn::TransformToolResult(State, Content, Tool)
				},
				Command::TransformTerminalOutput { Content } => {
					crate::Hook::Terminal::Entry::Fn::TransformTerminalOutput(State, Content)
				},
				Command::PreLLMCall => crate::Hook::LLM::Pre::Fn::Fn(State),
				Command::PostLLMCall => crate::Hook::LLM::Post::Fn::Fn(State),
				Command::Catalog { Mode } => crate::Catalog::Build::Fn::Fn(State, Mode),
				Command::Stats => {
					serde_json::json!({
						"version": env!("CARGO_PKG_VERSION"),
						"inline_entries": State.InlineStore.len(),
						"markers": State.RecentMarkers.len(),
						"turn": State.TurnCounter,
						"engine_enabled": State.ContextEngineEnabled,
					})
				},
				Command::Directive { Action, Name } => crate::Directive::Action::Fn(State, Action, Name),
				Command::Search { Query, TypeFilter } => {
					let Query = Query.to_lowercase();
					let Results:Vec<serde_json::Value> = State
						.RecentMarkers
						.iter()
						.filter(|Marker| {
							let MatchesQuery = Query.is_empty()
								|| Marker.Preview.to_lowercase().contains(&Query)
								|| Marker.CCRType.to_lowercase().contains(&Query);
							let MatchesType = TypeFilter.is_none_or(|Type| Marker.CCRType == Type);

							MatchesQuery && MatchesType
						})
						.take(20)
						.map(|Marker| {
							serde_json::json!({
								"hash":&Marker.hash,
								"type":Marker.CCRType,
								"size":Marker.Size,
								"preview":Marker.Preview,
							})
						})
						.collect();

					serde_json::json!({"total":Results.len(),"results":Results})
				},
				Command::Diff => {
					let Turns:Vec<serde_json::Value> = State
						.ConvIndex
						.iter()
						.map(
							|(Turn, (hash, Summary, Size))| serde_json::json!({"turn":Turn,"hash":hash,"summary":Summary,"size":Size}),
						)
						.collect();

					serde_json::json!({"turns":Turns,"total":Turns.len()})
				},
				Command::Files => {
					let Files:Vec<serde_json::Value> = State
						.ReferencedFiles
						.iter()
						.map(|(path, Tool)| serde_json::json!({"path":path,"tool":Tool}))
						.collect();

					serde_json::json!({"files":Files,"total":Files.len()})
				},
				Command::Classify { Content } => {
					let Ct = headroom_core::transforms::content_detector::detect_content_type(Content).content_type;

					serde_json::json!({"type":Ct.as_str(),"lines":Content.lines().count(),"bytes":Content.len()})
				},
				Command::Unknown { Name } => serde_json::json!({"error": format!("unknown hook: {}", Name)}),
				// `Prepare` consumes `Prefetch` into `Prepared::Prefetch`, so this
				// pair cannot be built by the export. The arm exists because the
				// type permits it: answering an error envelope keeps the function
				// total, where `unreachable!()` would abort the host process across
				// the FFI boundary over a state no caller can reach.
				Command::Prefetch { .. } => {
					serde_json::json!({"error": "internal error: prefetch reached the session lock unprepared"})
				},
			}
		},
	}
}
