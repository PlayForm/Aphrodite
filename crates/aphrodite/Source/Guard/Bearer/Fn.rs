//! The bearer-token allow/reject decision, with no axum types in sight.

/// The actual allow/reject decision, pulled out as a pure function per the
/// same pattern as [`CheckLoopbackRequest`] - directly unit-testable.
/// `configured = None` means auth is disabled; always `Ok`.
///
/// The rejection carries no request data on purpose: it becomes the `error`
/// field of a 401 body, and echoing the presented token back would put a
/// credential in a client-visible response.
pub fn CheckBearerToken(Configured:Option<&str>, AuthHeader:&str) -> anyhow::Result<()> {
	let Some(Token) = Configured else {
		return Ok(());
	};
	match AuthHeader.strip_prefix("Bearer ") == Some(Token) {
		true => Ok(()),
		false => anyhow::bail!("missing or invalid Authorization bearer token"),
	}
}
