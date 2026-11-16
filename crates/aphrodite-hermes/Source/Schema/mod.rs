//! Hermes tool schemas - JSON Schema definitions for all aphrodite tools.
//!
//! These are what the agent actually sees. Two consumers with very different
//! budgets read the same string, which drives the shape of every description
//! below:
//!
//! * The **deferred-catalog listing** (Hermes `tools/tool_search.py`,
//!   `_short_desc`) renders ONE line per tool: the first sentence, clipped to
//!   60 chars. So sentence one must be short, self-contained, and free of
//!   abbreviation periods ("e.g.") that would truncate it early.
//! * **`tool_describe`** returns the full record - but only
//!   `{name, description, parameters}`. It is a verbatim passthrough of the
//!   registered schema (`dispatch_tool_describe`), so any key we invent at
//!   the top level (`returns`, `examples`, `cost`) is silently dropped there
//!   AND forwarded into the OpenAI `function` object by
//!   `registry.get_definitions()`, where strict providers reject unknown
//!   fields. The return shape therefore lives INSIDE `description`, which is
//!   the only channel that reaches the model.
//!
//! Keyword choice is constrained by the sanitizers every schema passes
//! through: `tools/schema_sanitizer.py` (llama.cpp grammar safety),
//! `agent/gemini_schema.py` (Gemini `Schema` allowlist), and
//! `agent/anthropic_adapter.py::_normalize_tool_input_schema`. `enum`,
//! `default`, `minItems`, `minLength`, `pattern`, `items` and boolean
//! `additionalProperties` all survive that gauntlet; top-level
//! `anyOf`/`oneOf`/`allOf` do not, so mutually-exclusive arguments (hash vs
//! path) are expressed in prose instead of as a union.
//!
//! Layout: one leaf per tool schema (the directory names the tool family, the
//! item is `Fn`), plus `All/` (the ordered array), `Get/` (lookup by name) and
//! `Empty/` (the shared argument-less parameter block). The text of every
//! schema is a contract with the model: copy it, never reword it.

pub(crate) mod All;
mod Catalog;
mod Compress;
mod Diff;
mod Directive;
mod Empty;
mod File;
pub(crate) mod Get;
mod Prefetch;
mod PrefetchStatus;
mod Rebuild;
mod Reclassify;
mod Retrieve;
mod Search;
mod Smoke;
mod Stats;

#[cfg(debug_assertions)]
mod Debug;

#[cfg(test)]
mod Test;
