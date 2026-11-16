//! What happened to one directive file.
//!
//! Five filesystem facts collapse into three outcomes, and the collapse is the
//! point: a file is written, or it is already correct, or it was left alone AND
//! the operator has to be told why (user-modified content wins, or an I/O error).
//! The write path used to push into three `Vec`s from four `match` arms, which
//! made "which list did this file land in, and did it warn?" a question about
//! control flow rather than about the file.

pub mod Enum;
