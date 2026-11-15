//! Plain text: the line count, and the fallback for every unclassified type.

pub(crate) fn Fn(_Content:&str, LineCount:usize, Parts:&mut Vec<String>) { Parts.push(format!("ln={}", LineCount)); }
