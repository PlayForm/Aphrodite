//! A loaded script.

use std::{path::PathBuf, time::SystemTime};

/// A single loaded script file.
///
/// The source is kept, not the AST: a Rhai AST is not `Send`, and the engine is
/// held across hook calls on a shared engine instance. Recompiling per call is
/// the price of that, and it is why the mtime is stored - a compile is only paid
/// again when the file changed or a hook actually runs.
pub(crate) struct Struct {
	/// Where the script was loaded from (the identity in every warn line).
	pub(crate) path:PathBuf,
	/// Last-modified time at load, the reload trigger.
	pub(crate) Mtime:SystemTime,
	/// The script text, compiled per call.
	pub(crate) Source:String,
}
