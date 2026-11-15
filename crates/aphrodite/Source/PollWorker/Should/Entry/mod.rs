//! The two public backgrounding predicates.
//!
//! They differ only in what they can see: `ShouldBackground` runs after the tool
//! produced output and can therefore also apply the minimum-size floor, while
//! `ShouldBackgroundPre` runs before execution and only has the command. Both are
//! the same decision - a known-slow command - so neither may grow a second
//! heuristic of its own.

pub mod Fn;
