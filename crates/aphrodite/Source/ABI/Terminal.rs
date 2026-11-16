//! `AphroditeTerminal` - run the terminal-output hook for one payload.

use std::os::raw::c_char;

use crate::ABI::{Boundary::Answer::Enum::Enum as Answer, Guard::Content::Const::Maximum};

stateful!(
	/// Transform captured terminal output through the terminal hook and answer
	/// with the hook's JSON result.
	///
	/// Separate from `AphroditeTransform` because the terminal hook classifies
	/// and previews differently (exit codes and error lines are the signal, not
	/// file structure), and because `TransformTerminalOutput` has no tool name
	/// to key on.
	///
	/// # Safety
	/// `Content` must be null or point to a valid, NUL-terminated C string that
	/// stays readable and unmoved for the duration of this call. The returned
	/// pointer is owned by the caller and must be released with
	/// `AphroditeFreeString`.
	AphroditeTerminal,
	|s, Content:*const c_char| {
		crate::ABI::Guard::Reject::Fn::Fn(Content.len() > Maximum, || anyhow::anyhow!("content exceeds 16MB limit"))?;

		Ok(Answer::JSON(crate::Hook::Terminal::Entry::Fn::Fn(s, &Content)))
	}
);
