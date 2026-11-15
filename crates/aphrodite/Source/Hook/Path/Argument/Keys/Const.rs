//! The argument keys a path may be supplied under, in precedence order.

/// Argument keys that name a path, most specific first.
///
/// The ORDER is the contract, not a preference: `path` is the house spelling,
/// `file` and `file_path` are what other agent surfaces send, and `filename`
/// is the loosest. First match wins, so a call that supplies two of them is
/// read the same way every time instead of depending on JSON key order.
pub(crate) const Keys:&[&str] = &["path", "file", "file_path", "filename"];
