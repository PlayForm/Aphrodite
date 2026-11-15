//! Preview arms, one file per content type.
//!
//! The preview the model sees is the final, honest representation of the stored
//! payload, so each arm is a product surface in its own right: it promises what it
//! will surface about a type and must not promise more. The entry point resolves a
//! type and applies the length cap; the dispatch table maps a type to an arm.
//! Neither belongs in this facade, which only declares the arms and keeps the
//! pre-kind names alive for callers outside this module.

pub mod Build;
pub mod Cap;
pub mod Code;
pub mod CSV;
pub mod Diff;
pub mod Dispatch;
pub mod Entry;
pub mod Error;
pub mod Generic;
pub mod Git;
pub mod GitLog;
pub mod Grep;
pub mod HTML;
pub mod JSON;
pub mod Lint;
pub mod Log;
pub mod Ls;
pub mod Markdown;
pub mod Search;
pub mod Table;
pub mod Terminal;
pub mod Test;
pub mod XML;
pub mod YAML;
