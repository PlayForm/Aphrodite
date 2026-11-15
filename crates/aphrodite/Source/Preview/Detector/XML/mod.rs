//! HTML versus XML shape detection.
//!
//! Both shapes are tag documents, and the dedicated HTML arm extracts a `<title>` that XML does
//! not have, so the detector has to separate them. It is the only detector in the chain that
//! answers with a type rather than a yes or no, which is why the answer is a value with three
//! cases instead of two booleans that would admit an unreachable fourth.

pub mod Classify;
pub mod Document;
pub mod Fn;
