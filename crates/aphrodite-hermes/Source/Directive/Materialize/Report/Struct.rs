//! The provisioning report, as a value.

use std::path::PathBuf;

use serde_json::json;

/// What provisioning did, accumulated once and serialised once.
///
/// It exists so the JSON is built from a value rather than assembled at each
/// return: the early "could not create the directory" return used to carry its
/// own copy of the object literal, and a field added to one copy and not the
/// other is a report that lies on exactly the path an operator needs.
///
/// `status` is deliberately not a field. It is always `"ok"` - the attempt
/// completed - and every real failure lives in `Warnings`, so a caller that
/// needs to know whether the files are correct reads the warnings, never the
/// status.
pub(crate) struct Struct {
	/// The directory the attempt targeted, whether or not it could be created.
	pub(crate) Dir:PathBuf,
	/// Files written this run, sorted.
	pub(crate) Written:Vec<String>,
	/// Files left alone this run, sorted. A warned file is listed here too: it
	/// was skipped, and `Warnings` says why.
	pub(crate) Skipped:Vec<String>,
	/// Every degradation, in the order it happened.
	pub(crate) Warnings:Vec<String>,
}

impl Struct {
	/// The JSON report the C ABI returns.
	pub(crate) fn Json(&self) -> serde_json::Value {
		json!({
			"status": "ok",
			"dir": self.Dir.to_string_lossy(),
			"written": self.Written,
			"skipped": self.Skipped,
			"warnings": self.Warnings,
		})
	}
}
