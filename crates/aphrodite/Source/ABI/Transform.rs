//! `AphroditeTransform` - run the tool-result hook for one payload.

use std::os::raw::c_char;

use super::Guard::MaximumContent;

stateful!(
	/// Transform a tool result through the live hook pipeline and answer with
	/// the hook's JSON result.
	///
	/// This is the single-tool entry point; `AphroditeDispatch` covers the same
	/// hook by name. Oversize content is rejected before the hook sees it, so a
	/// host that pipes a runaway output here gets an error rather than a session
	/// whose store is full of something unretrievable.
	AphroditeTransform,
	|s, Content:*const c_char, Tool:*const c_char| {
		if Content.len() > MaximumContent {
			return serde_json::json!({"error":"content exceeds 16MB limit"});
		}
		crate::Hook::ToolResult::Entry::Fn::TransformToolResult(s, &Content, &Tool)
	}
);
