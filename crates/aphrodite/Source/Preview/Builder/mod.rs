//! Preview arms, one file per content type.
//!
//! The preview the model sees is the final, honest representation of the stored payload,
//! so each arm is a product surface in its own right: it promises what it will surface
//! about a type and must not promise more. The entry point resolves a type and applies the
//! length budget; the dispatch table maps a type to an arm. Neither belongs in this
//! facade, which only declares the arms.
//!
//! Six arms - `Git`, `GitLog`, `Grep`, `Search`, `Ls`, `Markdown` - are now a two-line
//! composition of a parser and a renderer under `Preview::Parse` and `Preview::Render`.
//! Each of them used to mix line recognition, aggregation, truncation and final formatting
//! in one function, so none of the four could be changed without the others. The remaining
//! arms are still single functions; the same split reaches them as their recognition rules
//! grow.

pub mod Build;
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
pub mod More;
pub mod Search;
pub mod Table;
pub mod Terminal;
pub mod Test;
pub mod XML;
pub mod YAML;
