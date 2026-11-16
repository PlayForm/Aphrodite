//! `aphrodite_hermes_proxy_health` - proxy liveness as JSON.

use std::os::raw::c_char;

use crate::{
	Guard::{Fn::guarded, ToJSON::Fn::Fn as to_json},
	Health::Proxy::Fn::proxy_health,
};

/// Probe proxy health via TCP connect, and report which build answered.
///
/// The payload carries the `build` stamp beside the two ports: after a dylib
/// rebuild the question is not "is it alive" but "is it the image I just built",
/// and a version cannot answer that.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_proxy_health() -> *mut c_char { guarded(|| to_json(&proxy_health())) }
