//! One builtin directive: its name and its embedded prose.
//!
//! The pair is a value rather than a `(String, String)` tuple because the two
//! strings are not interchangeable: `Name` becomes the file stem the core loader
//! derives the directive's name from, and `Content` is the prose written
//! verbatim. A tuple lets a caller swap them and still compile.

pub mod Struct;
