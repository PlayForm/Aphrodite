//! Resolving one knob through the precedence chain: runtime override → env
//! var → TOML key → hardcoded default.
//!
//! One file per knob family. The chains are the contract of this module and
//! are deliberately NOT factored into a shared helper: each family has a
//! different answer for "the value is present but unusable", and collapsing
//! them into one generic resolver is how those differences get lost.

pub mod Bool;
pub mod Limit;
pub mod List;
pub mod Number;
pub mod Section;
pub mod Size;
pub mod Template;
pub mod Text;
