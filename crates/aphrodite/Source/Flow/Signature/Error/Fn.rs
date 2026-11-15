//! FNV-1a signature of an error, built from its type and first message line.
//!
//! Line one only: distinct compiler errors differ there, so the signature
//! stays discriminating without over-matching. Hashing the whole message would
//! make every error unique - line numbers and paths change - and the
//! distinct-error count would stop meaning anything.

use crate::Flow::Hash::Fn::Fnv1a;

/// FNV-1a signature of an error: `ErrorType` + first line of `ErrorMessage`
/// (P2/P7). Distinct compiler errors differ on line 1, so this stays
/// discriminating without over-matching.
pub fn ErrorSig(ErrorType:Option<&str>, ErrorMessage:Option<&str>) -> u64 {
	let mut Buffer = String::new();
	Buffer.push_str(ErrorType.unwrap_or(""));
	Buffer.push('\u{1}');

	match ErrorMessage {
		Some(Message) => {
			Buffer.push_str(Message.lines().next().unwrap_or("").trim());
		},
		None => (),
	}

	Fnv1a(Buffer.as_bytes())
}
