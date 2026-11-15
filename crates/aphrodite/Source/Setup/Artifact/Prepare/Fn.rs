//! Applying the final permissions to a verified artifact.

use crate::Setup::Write::Permissions::Fn as SecurePerms;
use super::super::{Prepared, Verified};

/// Set the artifact's final mode while it is still staged.
///
/// The mode is a decision the caller made (binary `0o700`, dylib `0o755`), so it is
/// an argument rather than something derived here - a second derivation would be a
/// second opinion about it.
///
/// `Permissions::Fn` is already a no-op on non-Unix targets, so this call is
/// unconditional: Windows has no mode bits, and failing there would make setup
/// unusable rather than safer.
pub(crate) fn Fn(Verified:Verified::Struct::Struct, Mode:u32) -> anyhow::Result<Prepared::Struct::Struct> {
	SecurePerms(&Verified.Staged.Path, Mode)?;

	Ok(Prepared::Struct::Struct { Path:Verified.Staged.Path, Dest:Verified.Staged.Dest })
}
