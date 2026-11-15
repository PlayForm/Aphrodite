//! The guarded file read behind `aphrodite_retrieve(path=...)`.
//!
//! A security boundary, not a convenience reader: `Maintain/SECURITY.md` treats
//! the tool as a potential exfiltration channel, so the path is canonicalised,
//! required to sit under the current workspace, and capped. Moving this code must
//! not weaken any of the three.

pub mod Const;
pub mod Fn;
