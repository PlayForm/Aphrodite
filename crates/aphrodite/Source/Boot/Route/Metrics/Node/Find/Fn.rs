//! A path that may not resolve.

use serde_json::Value;

/// Walk `Path` with `get`, which answers `None` for a member the document does
/// not hold.
///
/// This is the lookup the optional counters use: no member, no line.
pub fn Fn<'a>(Path:&[&str], Stats:&'a Value) -> Option<&'a Value> {
	Path.iter().try_fold(Stats, |Node, Key| Node.get(*Key))
}
