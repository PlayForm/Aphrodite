//! Poll-worker auto-backgrounding: intercept slow tool calls and replace output
//! with a "poll worker" CCR marker so the agent can check progress asynchronously
//! via `process(action='poll')` instead of blocking on raw output.
//!
//! # Flow
//!
//! 1. A tool call (terminal, process, etc.) produces output.
//! 2. `Should::Entry` decides whether it is worth tracking; `Insert` builds the
//!    task and hands it to the `Registry`.
//! 3. `PreLLMCall` → `Check` selects the tasks worth a nudge and pushes one plan
//!    per task.
//! 4. When the agent calls `process(action='poll')`, `Update` decodes the result
//!    (`Decode`), applies it to the one task it names (`Apply`), and interprets the
//!    outcome (`Update::Interpret`) - storing the output and recording its marker
//!    on completion.
//! 5. `PostLLMCall` → `Expire` decides one retention fate per task and applies it.
//!
//! # The tree
//!
//! Every step of the flow owns one leaf directory, and a caller names the leaf
//! (`Registry::Admit::Fn::Fn`), so the file that defines an item is the only place
//! it is defined. The value types are the flow's vocabulary: `Event` (a decoded
//! poll), `Transition` (what it did), `Output` (the bounded accumulator), `Select`
//! (the two selections), `Apply`/`Completion` (the applied poll and its stored
//! form), and `Registry` (the set and the operations allowed to change it).
//!
//! Nothing outside `Registry` pushes, removes or retains: capacity, deduplication
//! and eviction order used to be re-derived by four modules and are now decisions
//! of one.

pub mod Apply;
pub mod Check;
pub mod Completion;
pub mod Decode;
pub mod Event;
pub mod Expire;
pub mod Insert;
pub mod Marker;
pub mod Output;
pub mod Registry;
pub mod Select;
pub mod Should;
pub mod Status;
pub mod Task;
pub mod Transition;
pub mod Update;

#[cfg(test)]
mod Test;
