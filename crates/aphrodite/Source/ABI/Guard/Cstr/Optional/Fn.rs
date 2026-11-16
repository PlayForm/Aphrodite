//! Resolving an optional argument.

use std::borrow::Cow;

use super::super::Enum::Enum as Decoded;

/// Resolve an argument the caller may legitimately omit.
///
/// `Absent` becomes the empty string, which is what every optional argument in
/// this ABI has always been read as: an absent `mode` is the catalogue's
/// default, an absent `hint` defers to the detector, an absent `action` is
/// `list`. Making that decision here rather than at each call site is what lets
/// the exports stop writing `unwrap_or_default()` over a value that also means
/// "too big".
///
/// `Oversize` is the one case that cannot be absorbed: the argument exists and no
/// export can serve it, so it is refused with the ABI's own cap string.
pub(crate) fn Fn<'a>(Decoded:Decoded<'a>) -> anyhow::Result<Cow<'a, str>> {
	match Decoded {
		Decoded::Absent => Ok(Cow::Borrowed("")),
		Decoded::Text(Text) => Ok(Text),
		Decoded::Oversize => anyhow::bail!("content exceeds 16MB limit"),
	}
}
