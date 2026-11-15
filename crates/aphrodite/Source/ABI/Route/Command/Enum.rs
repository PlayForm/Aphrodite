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
pub(crate) enum Enum<'a> {
	SessionStart,
	TransformToolResult { Content:&'a str, Tool:&'a str },
	TransformTerminalOutput { Content:&'a str },
	PreLLMCall,
	PostLLMCall,
	Catalog { Mode:&'a str },
	Stats,
	Directive { Action:&'a str, Name:&'a str },
	Search { Query:&'a str, TypeFilter:Option<&'a str> },
	Diff,
	Files,
	Classify { Content:&'a str },
	Prefetch { Paths:Vec<String> },
	Unknown { Name:&'a str },
}
