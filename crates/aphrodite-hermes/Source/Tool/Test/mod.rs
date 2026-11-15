//! Unit tests for tool dispatch, split by area.
//!
//! Each area owns one failure mode: the envelope heuristics (preview/type only),
//! the store-and-marker round trip (retrieval must stay lossless), the preview
//! collapse family, the guarded path read (a security boundary), and the per-tool
//! handlers whose output shape other code depends on.

mod Catalog;
mod Collapse;
mod Directive;
mod Registry;
mod Retrieve;
mod Roundtrip;
mod Stats;
mod Unwrap;
