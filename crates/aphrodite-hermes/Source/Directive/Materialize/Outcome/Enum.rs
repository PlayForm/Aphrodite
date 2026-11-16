//! What happened to one directive file.

/// The outcome of provisioning one directive file.
pub(crate) enum Enum {
	/// The file was missing and the embedded bytes are now on disk.
	Written(String),
	/// A byte-identical file was already there: nothing to do, nothing to say.
	Skipped(String),
	/// The file was left alone and the operator must know why.
	///
	/// `File` goes in the report's `skipped` array and `Message` in `warnings` -
	/// the two are carried together because an outcome that names a skipped file
	/// without its reason is exactly the silent failure this value prevents.
	Warned { File:String, Message:String },
}
