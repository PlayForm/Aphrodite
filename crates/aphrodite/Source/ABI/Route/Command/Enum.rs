//! The routing table as a value.

/// One decoded dispatch request.
///
/// This enum *is* the routing table: the thirteen literal arms the hook name
/// used to be matched against live here as variants, so `Run`'s match is
/// exhaustive and a hook the engine does not serve is the `Unknown` variant
/// rather than a fallthrough that has to invent an error.
///
/// The borrowed payloads are the caller's JSON args, not copies: a
/// `transform_tool_result` can carry 16 MB, and cloning it to satisfy a command
/// type would double the peak memory of every tool result for no gain. `Search`
/// keeps its query un-lowercased because the lowering is work the `search`
/// command does, and doing it here would run it for an invalid handle too.
///
/// `Prefetch` is the one variant that owns, because its paths are handed to a
/// reader that runs before the lock and must outlive the borrow of the args it
/// was built from.
///
/// The variant names are the hook names themselves (`transform_tool_result`,
/// `pre_llm_call`, ...), which is why they keep their compound spelling: they are
/// an external contract - the strings a host puts on the wire - not an internal
/// identifier that the path could qualify. `Route::Decode::Fn` maps each literal
/// to exactly one variant below.
pub(crate) enum Enum<'a> {
	/// `on_session_start` or `session_start`.
	SessionStart,
	/// `transform_tool_result`.
	TransformToolResult { Content:&'a str, Tool:&'a str },
	/// `transform_terminal_output`.
	TransformTerminalOutput { Content:&'a str },
	/// `pre_llm_call`.
	PreLLMCall,
	/// `post_llm_call`.
	PostLLMCall,
	/// `catalog`; `Mode` defaults to `"full"`.
	Catalog { Mode:&'a str },
	/// `stats`.
	Stats,
	/// `directive`; `Action` defaults to `"list"`, `Name` to the empty string.
	Directive { Action:&'a str, Name:&'a str },
	/// `search`; `Filter` is the optional type restriction.
	Search { Query:&'a str, Filter:Option<&'a str> },
	/// `diff`.
	Diff,
	/// `files`.
	Files,
	/// `classify`.
	Classify { Content:&'a str },
	/// `prefetch`; `Paths` is empty when the argument is absent or not an array.
	Prefetch { Paths:Vec<String> },
	/// A hook name the engine does not serve; `Name` is the name as received.
	Unknown { Name:&'a str },
}
