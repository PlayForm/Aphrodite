//! The management bearer-token decision.
//!
//! `Guard::Bearer::Fn` is the pure decision function (no env vars, no axum
//! `Request`/`Next`), so these tests need no env-var serialisation the way
//! `Guard::Token::Fn` itself would. Each test is named for the one case it
//! covers; the module path names the decision.

use crate::Guard::Bearer::Fn::Fn as Bearer;

/// Back-compat default: no token configured, so every request passes
/// regardless of what `Authorization` header (if any) is present.
#[test]
fn Unconfigured() {
	assert!(Bearer(None, "").is_ok());
	assert!(Bearer(None, "Bearer whatever").is_ok());
}

/// The header carries exactly the configured token.
#[test]
fn Matching() {
	assert!(Bearer(Some("secret123"), "Bearer secret123").is_ok());
}

/// No `Authorization` header at all.
#[test]
fn Absent() {
	assert!(Bearer(Some("secret123"), "").is_err());
}

/// A different token is rejected.
#[test]
fn Wrong() {
	assert!(Bearer(Some("secret123"), "Bearer wrong").is_err());
}

/// A raw token with no `Bearer ` scheme prefix must not pass, even when the
/// token value itself is correct - the header format is part of the contract.
#[test]
fn Unprefixed() {
	assert!(Bearer(Some("secret123"), "secret123").is_err());
}
