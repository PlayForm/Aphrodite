//! `Decode`: a hook name and its JSON args as a command.

use super::super::Command::Enum::Enum as Command;

/// Decode the hook name and args into a command, enforcing the payload cap.
///
/// The cap runs first and applies to every hook name, not only the ones that
/// carry content: a host that pipes a runaway output into `classify` gets the
/// same refusal as one that pipes it into `transform_tool_result`, because the
/// bytes are already materialised and the copy is what the cap exists to avoid.
///
/// The two `unwrap_or` defaults are contract values: an absent `tool_name` is
/// `"unknown"` (the span and the hook both need a label) and an absent `mode` /
/// `action` / `query` / `name` is the empty string or the documented default,
/// which is how the catalogue and the directive actions have always read an
/// omitted field.
///
/// Nothing here touches session state, so it all runs before the lock.
pub(crate) fn Fn<'a>(Name:&'a str, Args:&'a serde_json::Value) -> anyhow::Result<Command<'a>> {
	let Content = Args.get("content").and_then(|Value| Value.as_str()).unwrap_or("");

	crate::ABI::Guard::Reject::Fn::Fn(Content.len() > crate::ABI::Guard::Content::Const::Maximum, || {
		anyhow::anyhow!("content exceeds 16MB limit")
	})?;

	let Tool = Args.get("tool_name").and_then(|Value| Value.as_str()).unwrap_or("unknown");

	Ok(match Name {
		"on_session_start" | "session_start" => Command::SessionStart,
		"transform_tool_result" => Command::TransformToolResult { Content, Tool },
		"transform_terminal_output" => Command::TransformTerminalOutput { Content },
		"pre_llm_call" => Command::PreLLMCall,
		"post_llm_call" => Command::PostLLMCall,
		"catalog" => Command::Catalog { Mode:Args.get("mode").and_then(|Value| Value.as_str()).unwrap_or("full") },
		"stats" => Command::Stats,
		"directive" => {
			Command::Directive {
				Action:Args.get("action").and_then(|Value| Value.as_str()).unwrap_or("list"),
				Name:Args.get("name").and_then(|Value| Value.as_str()).unwrap_or(""),
			}
		},
		"search" => {
			Command::Search {
				Query:Args.get("query").and_then(|Value| Value.as_str()).unwrap_or(""),
				Filter:Args.get("type").and_then(|Value| Value.as_str()),
			}
		},
		"diff" => Command::Diff,
		"files" => Command::Files,
		"classify" => Command::Classify { Content },
		"prefetch" => {
			Command::Prefetch {
				Paths:Args
					.get("paths")
					.and_then(|Value| Value.as_array())
					.map(|Paths| Paths.iter().filter_map(|Value| Value.as_str().map(str::to_string)).collect())
					.unwrap_or_default(),
			}
		},
		_ => Command::Unknown { Name },
	})
}
