//! Hook bodies: the work behind every `aphrodite_hermes_call_hook` arm.
//!
//! Hermes fires hooks on a fixed contract, and each arm's RETURN SHAPE is that
//! contract: `transform_tool_result` / `transform_terminal_output` honor only a
//! *string* (first non-None string wins; non-strings pass through), so an arm
//! returns the CCR marker to replace output and `null` to leave it alone;
//! `pre_llm_call` returns `{"context": "..."}`; lifecycle hooks return a value
//! Hermes ignores.
//!
//! Leaves: `Call/` decodes one hook call and dispatches it, `Replacement/`
//! maps a compression payload to the value Hermes substitutes, `PostTool/` and
//! `Log/` are the dev-only debug-bridge lines (they never affect a return
//! value).

pub mod Call;
pub mod Log;
pub mod PostTool;
pub mod Replacement;

#[cfg(test)]
mod Test;
