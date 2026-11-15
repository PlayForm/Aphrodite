//! YAML shape detector.
//!
//! Three top-level key lines is a strong YAML signal, provided the keys are not
//! the log markers a compiler prints. This detector applies both rules. It is not
//! a YAML parser: values are never read.

use crate::Preview::{Input::Struct::Struct as Input, Line::YAMLKey};

/// True when the content looks like YAML.
///
/// Three or more top-level lowercase-key lines (`name: webapp`). Log-marker keys
/// (`error:`, `warning:`, `note:`, `info:`, `warn:`) are excluded by the
/// predicate, so a compiler log cannot be mis-tagged as YAML. It is not a
/// nested-structure check - only top-level keys vote.
pub(crate) fn Fn(Inp:&Input<'_>) -> bool {
	let YAMLKeys = Inp.raw.lines().filter(|l| YAMLKey::Fn(l)).count();
	YAMLKeys >= 3
}
