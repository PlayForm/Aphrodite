//! Markdown line predicates: heading lines and the structure that is not one.
//!
//! Both are prefix tests over one line, kept apart because a document is counted by
//! its headings while its remaining structure is counted as the lines between them.

pub mod Heading;
pub mod Structure;
