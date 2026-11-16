//! Resolving the requested smoke-test mode.

use serde_json::Value;

use crate::Tool::{Arg, Registry::Test::Mode};

/// The requested mode; `quick` when the argument is absent.
///
/// The absent value and the literal `quick` are the same request, so they are the
/// same arm - an empty `mode` is not "unknown", it is the default, and treating it
/// as `full` would silently triple the work of every default call.
pub(crate) fn Fn(Args:&Value) -> Mode::Enum::Enum {
	match Arg::Fn::Fn(Args, "mode") {
		"" | "quick" => Mode::Enum::Enum::Quick,
		_ => Mode::Enum::Enum::Full,
	}
}
