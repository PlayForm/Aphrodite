//! Poll-worker auto-backgrounding: intercept slow tool calls and replace
//! output with a "poll worker" CCR marker so the agent can check progress
//! asynchronously via `process(action='poll')` instead of blocking on raw
//! output.
//!
//! # Flow
//!
//! 1. A tool call (terminal, process, etc.) produces output.
//! 2. `TransformToolResult` / `TransformTerminalOutput` calls
//!    `should_background()` - if the heuristic fires, the output is NOT
//!    compressed normally. Instead, a `BgTask` is created and a special
//!    "poll worker" CCR marker replaces the output.
//! 3. `PreLLMCall` → `check_bg_tasks()` pushes ephemeral nudges telling
//!    the agent which tasks are running/done/failed.
//! 4. When the agent calls `process(action='poll')`, the result flows
//!    through `TransformToolResult` again - `update_from_poll()` matches
//!    it to a `BgTask` and updates status.
//! 5. On completion, the final output is compressed to CCR normally so
//!    `AphroditeRetrieve` can resolve it.
//! 6. `PostLLMCall` → `expire_stale_tasks()` drops tasks the agent
//!    stopped polling.
//!
//! This facade only declares. Each step of the flow owns one leaf directory,
//! and a caller names the leaf (`PollWorker::Insert::Fn::insert_bg_task`), so
//! the file that defines an item is the only place it is defined.

pub mod Check;
pub mod Expire;
pub mod Insert;
pub mod Marker;
pub mod Should;
pub mod Status;
pub mod Task;
pub mod Update;

#[cfg(test)]
mod Test;
