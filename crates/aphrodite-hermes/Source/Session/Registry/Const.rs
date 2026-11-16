//! The key a call with no session context falls back to.

/// Key for calls with no session context (tools, tests, empty session_id).
///
/// The value is `"default"`, and it is one constant rather than a literal at
/// each call site because every session-less call has to agree on it: a second
/// spelling would silently register a second session instead of joining the
/// fallback one.
pub(crate) const Fallback:&str = "default";
