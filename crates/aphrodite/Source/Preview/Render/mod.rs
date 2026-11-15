//! The final stage: a summary value in, the preview string out.
//!
//! A renderer owns exactly the decisions that are presentation - how many characters a
//! field may occupy, how a collection is joined, which wire word names the arm - and
//! owns no recognition at all. That split is what makes the acceptance rule and the
//! output shape change independently: a renderer edit can never move a line from one arm
//! to another.

pub mod Cap;
pub mod Git;
pub mod GitLog;
pub mod Listing;
pub mod Markdown;
pub mod Search;
