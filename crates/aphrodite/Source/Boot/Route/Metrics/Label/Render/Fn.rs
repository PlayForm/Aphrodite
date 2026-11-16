//! The label set, rendered as the `{key="value"}` suffix of a metric name.

use serde_json::Value;

use crate::Boot::Route::Metrics::{Label, Node};

/// The `{key="value"}` suffix, or the empty string for an unlabelled metric.
pub fn Fn(Label:&Label::Enum::Enum, Stats:&Value) -> Option<String> {
	match Label {
		Label::Enum::Enum::Plain => Some(String::new()),
		Label::Enum::Enum::Literal { Key, Value } => Some(format!("{{{Key}=\"{Value}\"}}")),
		Label::Enum::Enum::Document { Key, Path, Fallback } => {
			let Value = Node::Find::Fn::Fn(Path, Stats).and_then(Value::as_str).unwrap_or(*Fallback);
			Some(format!("{{{Key}=\"{Value}\"}}"))
		},
	}
}
