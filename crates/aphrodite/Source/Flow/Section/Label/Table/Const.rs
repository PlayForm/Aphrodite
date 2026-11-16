//! Which first line belongs to which section.
//!
//! A table, not a chain of tests: the label is a lookup in the order the
//! assembler emits the sections, and a new section adds one row instead of one
//! more guard. Order matters - a section whose first line starts with another's
//! prefix must be listed before it, which is why `[aphrodite: first-turn` is
//! last.

/// `(first-line prefix, label)` pairs, in match order.
pub(crate) const Prefixes:&[(&str, &str)] = &[
	("[directives", "directives"),
	("[nudge", "nudges"),
	("[engine-offload", "engine_offload"),
	("[retrieval", "retrieval"),
	("[auto-expand", "auto_expand"),
	("[recall]", "recall"),
	("[aphrodite: first-turn", "session_inject"),
];
