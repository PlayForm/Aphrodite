//! How much evidence the adaptation requires before it acts.
//!
//! The window is the ledger (`super::Const::Cap` events at most); this is the
//! floor below which the window is not read as a trend at all.

pub mod Const;
