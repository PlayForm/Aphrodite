//! One counter in the exposition.

use crate::Boot::Route::Metrics::{Kind, Label};

/// One counter: what it is called, where its number lives, and how it is written.
///
/// `Name` is the metric name verbatim - the exposition is a wire contract, so
/// nothing here is derived from the JSON key.
#[derive(Clone, Copy)]
pub struct Struct {
	/// The metric name, exactly as Prometheus must read it.
	pub Name:&'static str,
	/// The path into the stats document that holds the number.
	pub Path:&'static [&'static str],
	/// How the number is read and written.
	pub Kind:Kind::Enum::Enum,
	/// The label set rendered after the name.
	pub Label:Label::Enum::Enum,
}
