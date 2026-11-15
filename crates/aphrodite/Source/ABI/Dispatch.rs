//! `AphroditeDispatch` - the universal entry point every host hook routes
//! through.

use std::os::raw::c_char;

use crate::ABI::{
	Guard::{Cstr, MaximumContent, ToJSONError, ToJSONOk},
	Handle::Lookup::WithState,
};

/// Universal hook dispatcher. Python calls this for every hook handler.
/// Returns JSON-wrapped result or raw string if content-only.
///
/// The hook name is the routing key: the six hook names, the read-only
/// catalogue queries (`catalog`, `Stats`, `Diff`, `Files`, `search`,
/// `Classify`, `directive`) and `prefetch` all arrive here. An unrecognised name
/// answers `{"error":"unknown hook: ..."}` rather than an empty result, so a
/// host that misspells a hook learns it immediately.
///
/// Both `OnSessionStart` and the bare `session_start` are accepted, because
/// hosts in the wild register either; only the prefixed form appears in
/// `AphroditeHooks`.
#[unsafe(no_mangle)]
pub extern "C" fn AphroditeDispatch(
	Handle:*const c_char,
	HookName:*const c_char,
	ArgsJSON:*const c_char,
) -> *mut c_char {
	let Hid = match unsafe { Cstr(Handle) }.unwrap_or_default().parse::<usize>() {
		Ok(ID) => ID,
		Err(_) => return ToJSONError("invalid handle"),
	};
	let name = unsafe { Cstr(HookName) }.unwrap_or_default();
	let ArgsStr = unsafe { Cstr(ArgsJSON) }.unwrap_or_default();

	let Args:serde_json::Value = match serde_json::from_str(&ArgsStr) {
		Ok(v) => v,
		Err(e) => return ToJSONError(&format!("invalid args: {}", e)),
	};

	let Content = Args.get("content").and_then(|v| v.as_str()).unwrap_or("");
	if Content.len() > MaximumContent {
		return ToJSONError("content exceeds 16MB limit");
	}
	let Tool = Args.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");

	// F9: read prefetch files from disk BEFORE taking the HANDLES lock, not
	// inside `WithState`'s closure - up to 10MB per file was previously
	// read while holding the *global* handle-map mutex, serializing every
	// other session's every call behind this one's (possibly slow/cold-mount)
	// disk reads.
	// RefCell, not a plain local: `WithState`'s closure below still borrows
	// `Args`/`Content`/`Tool` for the other match arms, so it can't also
	// `move` this out of an owned `Option` - a `RefCell` lets the "prefetch"
	// arm take it through a shared capture instead.
	let PrefetchOutcomes = std::cell::RefCell::new(if name == "prefetch" {
		let Paths = Args.get("paths").and_then(|v| v.as_array()).cloned().unwrap_or_default();
		let PathStrings:Vec<String> = Paths.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect();
		Some(crate::Prefetch::Read::Entry::Fn::ReadPaths(&PathStrings))
	} else {
		None
	});

	match WithState(Hid, |s| {
		match name.as_ref() {
			"on_session_start" | "session_start" => crate::Hook::Session::Fn::Fn(s),
			"transform_tool_result" => crate::Hook::ToolResult::Entry::Fn::TransformToolResult(s, Content, Tool),
			"transform_terminal_output" => crate::Hook::Terminal::Entry::Fn::TransformTerminalOutput(s, Content),
			"pre_llm_call" => crate::Hook::LLM::Pre::Fn::Fn(s),
			"post_llm_call" => crate::Hook::LLM::Post::Fn::Fn(s),
			"catalog" => {
				let Mode = Args.get("mode").and_then(|v| v.as_str()).unwrap_or("full");
				crate::Catalog::Build::Fn::Fn(s, Mode)
			},
			"stats" => {
				serde_json::json!({
					"version": env!("CARGO_PKG_VERSION"),
					"inline_entries": s.InlineStore.len(),
					"markers": s.RecentMarkers.len(),
					"turn": s.TurnCounter,
					"engine_enabled": s.ContextEngineEnabled,
				})
			},
			"directive" => {
				let Action = Args.get("action").and_then(|v| v.as_str()).unwrap_or("list");
				let name = Args.get("name").and_then(|v| v.as_str()).unwrap_or("");
				crate::Directive::Action::Fn(s, Action, name)
			},
			"search" => {
				let Query = Args.get("query").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
				let TypeFilter = Args.get("type").and_then(|v| v.as_str());
				let Results:Vec<serde_json::Value> = s
					.RecentMarkers
					.iter()
					.filter(|m| {
						let MatchesQuery = Query.is_empty()
							|| m.Preview.to_lowercase().contains(&Query)
							|| m.CCRType.to_lowercase().contains(&Query);
						let MatchesType = TypeFilter.is_none_or(|t| m.CCRType == t);
						MatchesQuery && MatchesType
					})
					.take(20)
					.map(|m| serde_json::json!({"hash":&m.hash,"type":m.CCRType,"size":m.Size,"preview":m.Preview}))
					.collect();
				serde_json::json!({"total":Results.len(),"results":Results})
			},
			"diff" => {
				let Turns:Vec<serde_json::Value> = s
					.ConvIndex
					.iter()
					.map(
						|(Turn, (hash, Summary, Size))| serde_json::json!({"turn":Turn,"hash":hash,"summary":Summary,"size":Size}),
					)
					.collect();
				serde_json::json!({"turns":Turns,"total":Turns.len()})
			},
			"files" => {
				let Files:Vec<serde_json::Value> = s
					.ReferencedFiles
					.iter()
					.map(|(path, Tool)| serde_json::json!({"path":path,"tool":Tool}))
					.collect();
				serde_json::json!({"files":Files,"total":Files.len()})
			},
			"classify" => {
				let Ct = headroom_core::transforms::content_detector::detect_content_type(Content).content_type;
				serde_json::json!({"type":Ct.as_str(),"lines":Content.lines().count(),"bytes":Content.len()})
			},
			"prefetch" => {
				// `PrefetchOutcomes` is always `Some` here because
				// `name == "prefetch"` is exactly the condition that set it
				// above, before this closure runs.
				let Outcomes = PrefetchOutcomes
					.borrow_mut()
					.take()
					.expect("prefetch_outcomes set when name == \"prefetch\"");
				crate::Prefetch::Insert::Fn::InsertOutcomes(s, Outcomes)
			},
			_ => serde_json::json!({"error": format!("unknown hook: {}", name)}),
		}
	}) {
		Ok(v) => ToJSONOk(&v),
		Err(e) => ToJSONError(&format!("{e:#}")),
	}
}
