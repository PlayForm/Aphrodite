//! Resolve a path from a call's own arguments.

use crate::Hook::Path::{
	Argument::Keys::Const::Keys,
	Resolved::Struct::Struct as Resolved,
	Source::Enum::Enum as Source,
};

/// The first argument key that carries a non-empty path, in `Keys` order.
///
/// Non-empty rather than merely present: an agent that sends `"file": ""` has
/// named nothing, and treating the empty string as a path would record a
/// reference to the current directory.
pub(crate) fn Fn(Args:Option<&serde_json::Value>) -> Option<Resolved> {
	let Args = Args?;
	let (Key, Path) = Keys.iter().find_map(|Key| {
		Args.get(*Key)
			.and_then(|Value| Value.as_str())
			.filter(|Path| !Path.is_empty())
			.map(|Path| (*Key, Path))
	})?;

	Some(Resolved { Path:Path.to_string(), Source:Source::Argument { Key } })
}
