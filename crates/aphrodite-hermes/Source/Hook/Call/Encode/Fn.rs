//! Serialise a hook result into the string Hermes reads back.

use crate::Guard::Fn::to_c_string;

/// Serialise a hook result and hand it to the caller as an owned C string.
///
/// `null` serialises to the four characters `null`, which is what the Python
/// shim parses as "no replacement" - it is not a null POINTER. A serialisation
/// failure (impossible for `serde_json::Value`, but not statically) degrades to
/// the empty string rather than panicking.
pub(crate) fn encode(result:&serde_json::Value) -> *mut std::os::raw::c_char {
	to_c_string(&serde_json::to_string(result).unwrap_or_default())
}
