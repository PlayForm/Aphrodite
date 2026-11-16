//! One rendered signature line, capped.

use crate::StructExtract::Format::Cap::Fn::Fn as Cap;

/// Render `Kind` and `Text` as one signature line, truncated to the line cap.
///
/// The two parts are joined with a single space and trimmed, then measured once by `Format::Cap` -
/// so a signature never exceeds `MaximumSignatureLength`, and a line that already fits is rendered
/// whole. It is not a language-aware formatter: the caller passes the already-rendered text
/// (`struct Foo`, `type Bar`) and this only joins and caps.
pub(crate) fn Fn(Kind:&str, Text:&str) -> String { Cap(format!("{Kind} {Text}").trim().to_string()) }
