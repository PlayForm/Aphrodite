//! Resolving the retrieve target from the args.

use serde_json::Value;

use crate::Tool::{Arg, Registry::Retrieve::Target};

/// The target the caller asked for, or `None` when it asked for neither.
///
/// `path` is tested first and wins: a call carrying both is a path read, which is
/// the documented rule, and encoding it here means the handler cannot apply the
/// other order. `None` is the ABSENCE case - no argument at all - and it is
/// distinct from a read that failed, which is what the handler reports as
/// `found: false`.
pub(crate) fn Fn(Args:&Value) -> Option<Target::Enum::Enum<'_>> {
	let Path = Arg::Fn::Fn(Args, "path");
	let Hash = Arg::Fn::Fn(Args, "hash");

	(!Path.is_empty())
		.then_some(Target::Enum::Enum::Path(Path))
		.or_else(|| (!Hash.is_empty()).then_some(Target::Enum::Enum::Hash(Hash)))
}
