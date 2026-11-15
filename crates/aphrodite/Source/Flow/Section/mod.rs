//! How the assembler names, joins and counts a section.

pub mod Label;

pub(crate) use self::Label::{JoinSections, NudgesPresent, SectionLabel};
