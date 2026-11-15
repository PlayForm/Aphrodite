//! The upstream API key, wrapped so it cannot reach a log line.
//!
//! `Secret` is a newtype, not a bare `String`: `Debug` and `Display` both
//! print `[REDACTED]`, so a `{:?}` over any struct that carries the key -
//! `AppState` above all, whose whole field set is debug-printable - cannot
//! leak it. `expose()` is the one deliberate way out, used only to build the
//! upstream `Authorization` header. A wrapper that printed its value would be
//! worse than no wrapper: it invites the log line it was written to prevent.

/// API key wrapper with safe Debug and Display - never leaks to logs.
#[derive(Clone)]
pub struct Struct(pub(crate) String);

impl std::fmt::Debug for Struct {
	fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "[REDACTED]") }
}

impl std::fmt::Display for Struct {
	fn fmt(&self, f:&mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "[REDACTED]") }
}

impl Struct {
	/// Expose the raw API key value for use in HTTP headers.
	pub fn Expose(&self) -> &str { &self.0 }
}

impl From<&str> for Struct {
	fn from(s:&str) -> Self { Struct(s.to_string()) }
}

impl From<String> for Struct {
	fn from(s:String) -> Self { Struct(s) }
}
