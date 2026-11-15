//! Integer-preceding-keyword parse.
//!
//! Counts in tool output are written as `<number> <keyword>` - `220 passed`,
//! `3 failed`, `12 files`. Recovering the number means reading backwards from
//! the keyword to the digit run in front of it. This module owns both readings:
//! an exact match, and a case-insensitive one. It is not a tokenizer and it
//! does not validate that the keyword is a count word - the caller chooses the
//! keyword, so a wrong keyword yields 0 rather than a wrong number.

pub mod Casefold;
pub mod Exact;
