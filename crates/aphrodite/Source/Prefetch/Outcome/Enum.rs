//! What reading one path produced, before any state is touched.
//!
//! The enum is the reason the read and the insert can be split: a refusal, a
//! missing file, an oversize file and a loaded file are decided while no lock
//! is held, and the insert pass only classifies and stores. `Refused` carries
//! its reason because the refusal is a security decision the caller must be
//! able to report - a silent skip would look identical to a missing file.

/// Outcome of reading one path, before any state mutation.
pub enum Enum {
	/// The path is absent, unreadable, or not a file the reader could stat -
	/// nothing was read and nothing will be stored.
	Missing,
	/// Path refused by the prefetch guard (`.env` / `auth.json` / `~/.ssh` /
	/// Hermes home / non-file) - never read, never stored.
	Refused { Reason:String },
	/// The path exists but could not be read (permission, encoding, I/O).
	Error,
	/// The path is readable but exceeds the per-file prefetch limit; the size
	/// is carried so the caller can report the real number it refused.
	SkippedSize { Size:u64 },
	/// The file was read in full. The size the filesystem reported is not carried:
	/// a file can change between the size check and the read, so the only length that
	/// means anything is the one `Prepare` measures from the bytes it actually holds.
	Loaded { Content:String },
}
