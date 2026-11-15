//! Markdown document preview arm.
//!
//! Parse each heading, aggregate the per-level counts and the first heading, then render
//! them. It is not a renderer: inline formatting, code fences and link targets are not
//! interpreted.

use crate::Preview::{Input::Struct::Struct as Input, Parse, Render};

/// Build a preview of a markdown document.
///
/// Promises a heading tally by level plus the first heading text. It is not a table of
/// contents: only headings are counted, so a document that is all prose previews as zero
/// headings rather than as unreadable.
pub(crate) fn Fn(Inp:&Input<'_>) -> String { Render::Markdown::Fn::Fn(&Parse::Markdown::Fn::Fn(Inp), Inp.Total) }
