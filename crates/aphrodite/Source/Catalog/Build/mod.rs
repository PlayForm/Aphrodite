//! The catalog builder.
//!
//! `Fn/` is the JSON projection, `Enrich/` is the fields only a TOC catalog carries and
//! `Recommendation/` builds the retrieve suggestions that enrichment adds. The split
//! keeps the "TOC or not" decision in one place: it used to be three `== "toc"` tests
//! in the builder, the last of which also had to restate whether the catalog was empty.

pub mod Enrich;
pub mod Fn;
pub mod Recommendation;
