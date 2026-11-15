//! The `[compression] auto_expand` consumer.
//!
//! Two questions, one file each: does this content qualify
//! (`Applies/`), and what does the resolved content look like (`Content/`).
//! Auto-expand is inert until a configuration turns it on; when it is on it
//! must be invisible to the model - no `<<<CCR:...>>>` marker may reach it.

pub mod Applies;
pub mod Content;
