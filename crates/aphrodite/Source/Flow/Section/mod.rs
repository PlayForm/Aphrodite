//! How the assembler names, joins and counts a section.
//!
//! Three small questions, three leaves: `Label` names a section for the debug
//! line, `Join` produces the exact string the budget is measured against, and
//! `Present` answers whether the never-drop nudge block exists this turn.

pub mod Join;
pub mod Label;
pub mod Present;
