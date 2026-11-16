//! `aphrodite_hermes_version` - this crate's version.

use std::os::raw::c_char;

use crate::Guard::{Fn::guarded, ToJSON::Fn::Fn as to_json};

/// Version of this crate.
///
/// The ABI's version is the crate semver and stays that way; WHICH BUILD is
/// answering is `aphrodite_hermes_proxy_health`'s `build` stamp, which names the
/// revision instead of a version two different images can share.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_version() -> *mut c_char {
	guarded(|| to_json(&serde_json::json!({"version": env!("CARGO_PKG_VERSION")})))
}
