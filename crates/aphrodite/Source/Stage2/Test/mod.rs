//! Test battery for stage-2 compression, split by area.
//!
//! `Compress.rs` covers the three gates on the public entry point;
//! `Reduce.rs` covers the reducers themselves. The split follows the module
//! tree so a failing gate and a failing reducer never share a file.

mod Compress;
mod Reduce;
