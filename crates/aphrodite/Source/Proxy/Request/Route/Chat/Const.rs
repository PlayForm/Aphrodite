//! `POST /v1/chat/completions` - the one path the compression pass understands.

/// The Chat Completions endpoint path, leading slash included: the OpenAI wire
/// contract, so it is a literal and not a configuration key. `Request::Route`
/// normalises both sides with `trim_start_matches('/')` before comparing, so a
/// path arriving without the leading slash still classifies as this route.
pub(crate) const Path:&str = "/v1/chat/completions";
