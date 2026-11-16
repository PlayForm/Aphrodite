//! The number, as the exposition writes it.

use serde_json::Value;

use crate::Boot::Route::Metrics::{Kind, Node};

/// The number at `Path`, written the way `Kind` says, or `None` when the
/// document does not hold it.
pub fn Fn(Kind:Kind::Enum::Enum, Path:&[&str], Stats:&Value) -> Option<String> {
	match Kind {
		Kind::Enum::Enum::Shown => Some(Node::Read::Fn::Fn(Path, Stats).to_string()),
		Kind::Enum::Enum::Count => {
			Node::Find::Fn::Fn(Path, Stats)
				.and_then(Value::as_u64)
				.map(|Count| Count.to_string())
		},
		Kind::Enum::Enum::Ratio => {
			Node::Find::Fn::Fn(Path, Stats)
				.and_then(Value::as_f64)
				.map(|Ratio| format!("{Ratio:.2}"))
		},
		Kind::Enum::Enum::Seconds => {
			Node::Find::Fn::Fn(Path, Stats)
				.and_then(Value::as_u64)
				.map(|Micros| format!("{:.6}", Micros as f64 / 1_000_000.0))
		},
	}
}
