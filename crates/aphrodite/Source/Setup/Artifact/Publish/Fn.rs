//! Moving a prepared artifact to its final path.

use super::super::{Prepared, Published};

/// Rename the staged artifact onto its destination.
///
/// The staging path is a sibling of the destination, so this rename never crosses a
/// filesystem - which is the reason `Stage::Fn` puts the partial file there rather
/// than in a temporary directory. A rename is atomic, so the destination holds
/// either the previous artifact or the new one, never a half-written file.
///
/// It is not a transaction across artifacts. Publishing the binary and publishing
/// two dylibs are three renames, and a failure between them leaves a `binaries/`
/// directory with a new binary and an old dylib. Setup keeps no journal and no
/// backup, so that state cannot be rolled back - re-running is the repair.
pub(crate) fn Fn(Prepared:Prepared::Struct::Struct) -> anyhow::Result<Published::Struct::Struct> {
	match std::fs::rename(&Prepared.Path, &Prepared.Dest) {
		Ok(()) => Ok(Published::Struct::Struct { Path:Prepared.Dest }),
		Err(e) => {
			Err(anyhow::Error::new(e).context(format!(
				"publish {} to {}",
				Prepared.Path.display(),
				Prepared.Dest.display()
			)))
		},
	}
}
