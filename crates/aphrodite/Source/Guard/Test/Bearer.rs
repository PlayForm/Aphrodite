//! The management bearer-token decision.
//!
//! `CheckBearerToken` is the pure decision function (no env vars, no axum
//! `Request`/`Next`), so these tests need no env-var serialisation the way
//! `mgmt_token()` itself would.

use Guard::Bearer::Fn::CheckBearerToken;

#[test]
fn TestCheckBearerTokenPassesWhenUnconfigured() {
	// Back-compat default: no token configured -> every request passes,
	// regardless of what Authorization header (if any) is present.
	assert!(CheckBearerToken(None, "").is_ok());
	assert!(CheckBearerToken(None, "Bearer whatever").is_ok());
}

#[test]
fn TestCheckBearerTokenAcceptsMatchingToken() {
	assert!(CheckBearerToken(Some("secret123"), "Bearer secret123").is_ok());
}

#[test]
fn TestCheckBearerTokenRejectsMissingHeader() {
	assert!(CheckBearerToken(Some("secret123"), "").is_err());
}

#[test]
fn TestCheckBearerTokenRejectsWrongToken() {
	assert!(CheckBearerToken(Some("secret123"), "Bearer wrong").is_err());
}

#[test]
fn TestCheckBearerTokenRejectsMissingBearerPrefix() {
	// A raw token with no "Bearer " scheme prefix must not pass, even if
	// the token value itself is correct - the header format matters.
	assert!(CheckBearerToken(Some("secret123"), "secret123").is_err());
}
