//! Tests for the configuration loader, split by the verb under test: `Load`
//! (file resolution), `Resolve` (precedence), `Apply` (section → state) and
//! `Warn` (the parse-failure diagnostic).
//!
//! `Guard` holds the two process-global serializers the areas share - cwd and
//! env are process state, so the tests that touch them cannot run in
//! parallel with each other.

mod Apply;
mod Guard;
mod Load;
mod Resolve;
mod Warn;
