//! Resolving an argument the caller must supply.

use std::borrow::Cow;

use super::super::Enum::Enum as Decoded;

/// Resolve an argument whose absence is a refusal.
///
/// The message is the ABI's one "no payload at all" string and it is a contract:
/// `AphroditeClassify`, `AphroditePreview`, `AphroditeStage2`,
/// `AphroditeStructExtract` and `AphroditeFilterLines` have always answered
/// `null content` for a null content pointer, and a host may match on it. It
/// lives here so the five exports cannot drift apart.
///
/// The distinction this preserves is the reason the leaf exists: a null pointer
/// and an empty string are different mistakes, and a host debugging its own
/// marshalling needs to tell "I passed no pointer" from "I passed a zero-length
/// string".
pub(crate) fn Fn<'a>(Decoded:Decoded<'a>) -> anyhow::Result<Cow<'a, str>> {
	match Decoded {
		Decoded::Absent => anyhow::bail!("null content"),
		Decoded::Text(Text) => Ok(Text),
		Decoded::Oversize => anyhow::bail!("content exceeds 16MB limit"),
	}
}
