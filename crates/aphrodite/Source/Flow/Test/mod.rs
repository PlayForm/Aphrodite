//! Flow tests, split by behaviour family.
//!
//! One file per thing that can independently break: budget survival, the
//! prompt renderer, each opt-in guidance section, the engine trigger, the
//! always-present sections (first-turn orientation, poll-worker status), and
//! the two telemetry primitives. The fixtures live in `Support`; nothing else
//! is shared between areas, so a change in one area cannot mask a regression
//! in another.

mod Support;

mod AutoExpand;
mod Budget;
mod EngineOffload;
mod FirstTurn;
mod PollStatus;
mod Prompt;
mod Retrieval;
mod Signature;
mod Window;
