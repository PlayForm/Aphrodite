//! A path that always answers.

use serde_json::Value;

/// Walk `Path` with `Index`, which answers `Null` for a member the document does
/// not hold instead of failing.
///
/// This is the lookup the always-present counters use: the line is written
/// whether or not the number is there, exactly as before.
pub fn Fn<'a>(Path:&[&str], Stats:&'a Value) -> &'a Value { Path.iter().fold(Stats, |Node, Key| &Node[*Key]) }
