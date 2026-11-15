//! The bearer-token allow/reject decision, with no axum types in sight.

/// The actual allow/reject decision, pulled out as a pure function per the
/// same pattern as [`CheckLoopbackRequest`] - directly unit-testable.
/// `configured = None` means auth is disabled; always `Ok`.
pub fn CheckBearerToken(Configured:Option<&str>, AuthHeader:&str) -> Result<(), &'static str> {
	let Some(Token) = Configured else {
		return Ok(());
	};
	if AuthHeader.strip_prefix("Bearer ") == Some(Token) {
		Ok(())
	} else {
		Err("missing or invalid Authorization bearer token")
	}
}
