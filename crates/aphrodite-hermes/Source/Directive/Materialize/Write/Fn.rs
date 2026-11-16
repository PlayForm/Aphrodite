//! The per-file write decision.

use std::{fs, path::Path};

use crate::Directive::{Entry, Materialize::Outcome};

/// Provision ONE directive file into `Dir`, and say what happened to it.
///
/// Non-destructive by construction: an existing file is only replaced when its
/// bytes are the embedded bytes, which is a no-op; anything else is left as the
/// user wrote it. Every I/O failure becomes a `Warned` outcome rather than an
/// error, because provisioning can never fail the caller - the loader only needs
/// the files that are missing.
///
/// The read comes first, so a directory that is unreadable for a reason other
/// than "file absent" is reported as such instead of being treated as absent and
/// then failing the write.
pub(crate) fn Fn(Dir:&Path, Entry:&Entry::Struct::Struct) -> Outcome::Enum::Enum {
	let File = format!("{}.md", Entry.Name);
	let Destination = Dir.join(&File);

	match fs::read(&Destination) {
		Ok(Existing) if Existing == Entry.Content.as_bytes() => Outcome::Enum::Enum::Skipped(File),
		Ok(_) => {
			Outcome::Enum::Enum::Warned {
				Message:format!(
					"directives/{File} exists with different content; leaving user-modified file as-is (not \
					 overwritten)"
				),
				File,
			}
		},
		Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
			match fs::write(&Destination, Entry.Content.as_bytes()) {
				Ok(()) => Outcome::Enum::Enum::Written(File),
				Err(Error) => {
					Outcome::Enum::Enum::Warned { Message:format!("could not write directives/{File}: {Error}"), File }
				},
			}
		},
		Err(e) => {
			Outcome::Enum::Enum::Warned {
				Message:format!("could not read directives/{File}: {e}; leaving as-is"),
				File,
			}
		},
	}
}
