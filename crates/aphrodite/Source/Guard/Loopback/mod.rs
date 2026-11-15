//! Loopback enforcement: the peer-IP check plus the `Host` allowlist that makes
//! it DNS-rebinding-safe.
//!
//! Both layers are required, and neither is a substitute for the other. The
//! peer check alone lets a hostile page through whose hostname DNS-resolves to
//! 127.0.0.1 - that connection genuinely is loopback; the `Host` check alone
//! would not stop a remote peer that guesses a loopback hostname.

pub mod Fn;
pub mod Middleware;

