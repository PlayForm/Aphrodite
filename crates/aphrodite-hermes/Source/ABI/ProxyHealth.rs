//! `aphrodite_hermes_proxy_health` - proxy liveness as JSON.

use std::os::raw::c_char;

use crate::{
	Guard::Fn::{guarded, to_c_string},
	Health::Proxy::Fn::proxy_health,
};

/// Probe proxy health via TCP connect.
#[unsafe(no_mangle)]
pub extern "C" fn aphrodite_hermes_proxy_health() -> *mut c_char {
	guarded(|| to_c_string(&proxy_health().to_string()))
}
