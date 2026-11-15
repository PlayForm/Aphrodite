//! `AphroditeTransform` - run the tool-result hook for one payload.

use std::os::raw::c_char;

use crate::ABI::{Boundary::Answer::Enum::Enum as Answer, Guard::Content::Const::MaximumContent};

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
		crate::ABI::Guard::Reject::Fn::Fn(Content.len() > MaximumContent, || {
			anyhow::anyhow!("content exceeds 16MB limit")
		})?;

		Ok(Answer::JSON(crate::Hook::ToolResult::Entry::Fn::TransformToolResult(
			s, &Content, &Tool,
		)))
	}
);
