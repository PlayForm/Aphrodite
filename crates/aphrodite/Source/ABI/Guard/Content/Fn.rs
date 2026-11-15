//! `CheckContent`: the three ordered refusals a stored payload must survive.

use super::Const::MaximumContent;

/// Reject a payload no export can serve: empty, over the 16 MB cap, or
/// carrying an interior NUL that would silently truncate the C string handed
/// back to the caller.
///
/// The three refusals are distinct messages on purpose - a host marshalling its
/// own buffers needs to tell "sent nothing" from "sent too much" from "sent a
/// NUL". The caller renders them through `format!("{e:#}")` into the JSON error
/// payload, so the message is the whole diagnosis the C side gets, and the three
/// strings below are the C side's entire vocabulary for it.
///
/// The order is the policy, not an accident: an empty payload is reported as
/// empty even though it is also under the cap, and a NUL is reported as a NUL
/// only once the size is known to be servable. Three `?`-composed rejections
/// keep that order readable and each error is constructed only when it rejects.
pub(crate) fn Fn(Content:&str) -> anyhow::Result<()> {
	crate::ABI::Guard::Reject::Fn::Fn(Content.is_empty(), || anyhow::anyhow!("empty content"))?;
	crate::ABI::Guard::Reject::Fn::Fn(Content.len() > MaximumContent, || {
		anyhow::anyhow!("content exceeds 16MB limit")
	})?;
	crate::ABI::Guard::Reject::Fn::Fn(Content.contains('\0'), || anyhow::anyhow!("content contains null bytes"))
}
