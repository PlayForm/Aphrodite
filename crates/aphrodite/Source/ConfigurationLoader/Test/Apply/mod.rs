//! `Apply` tests, split by the section each applier writes.
//!
//! The area split is `Load`/`Resolve`/`Apply`/`Warn`; `Apply` is large enough
//! that it nests once more, one file per applier under test.

mod Compression;
mod Directive;
mod Preview;
mod Prompt;
mod Template;
