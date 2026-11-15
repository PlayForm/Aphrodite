//! Assemble the terminal failure.
//!
//! The context string is unchanged from the loop it replaces - it is what
//! `/stats`'s `LastErrors` shows an operator, and rewording it here would make
//! a past incident unsearchable.

use crate::Proxy::Handle::Proxy::Forward::Failure::{Kind::Enum::Enum as FailureKind, Struct::Struct as FailureStruct};

/// Build a failure from the transport error that ended the loop.
pub(crate) fn Fn(Kind:FailureKind, Attempts:u32, Error:reqwest::Error, Url:&str) -> FailureStruct {
	FailureStruct {
		Kind,
		Attempts,
		Source:anyhow::Error::new(Error).context(format!("send the upstream request to {Url}")),
	}
}
