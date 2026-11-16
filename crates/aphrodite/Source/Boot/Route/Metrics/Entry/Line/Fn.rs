//! One counter line.

use serde_json::Value;

use crate::Boot::Route::Metrics::{Entry, Kind, Label};

/// One exposition line: `name{labels} value`.
///
/// `None` means the stats document does not hold this counter at all, which is
/// how an absent section contributes no line.
pub fn Fn(Entry:&Entry::Struct::Struct, Stats:&Value) -> Option<String> {
	let Number = Kind::Value::Fn::Fn(Entry.Kind, Entry.Path, Stats)?;
	let Attached = Label::Render::Fn::Fn(&Entry.Label, Stats)?;
	Some(format!("{}{} {}\n", Entry.Name, Attached, Number))
}
