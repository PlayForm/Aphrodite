//! The shared content view every detector and arm reads.
//!
//! One blob, one view, many readers - the module exists so the line split and the
//! counts happen once instead of once per detector.

pub mod Struct;
