//! Per-turn injected context - the single choke point every injection site
//! routes through.
//!
//! Before this module, three call sites composed the model-visible context
//! independently: the Hermes bridge `PreLLMCall` arm, the
//! `context_engine_pre_llm` tool, and core `Hook::LLM::Pre::Fn::Fn`. The bridge
//! sites never called `BuildDirectiveContext`, so the directives system was
//! invisible on the only path real sessions use (bug class 01-F3 / 04-F1).
//! Routing every site through one assembler makes the "bridge reimplements the
//! hook body and forks" failure mode unrepresentable.
//!
//! The tree is grouped by what a section IS, not by where it renders:
//! `Prompt/` owns the template strings and their renderer, `Retrieval/` and
//! `Engine/` own the opt-in guidance sections, `Nudge/` owns the ephemeral
//! one-shots, `FirstTurn/` the one-shot orientation, `TurnContext/` the
//! assembler that orders and budgets them, `Section/` the small helpers the
//! assembler needs to talk about a section, and `Hash/`, `Signature/`,
//! `Window/` the turn-telemetry primitives. This facade declares and
//! re-exports; it defines nothing.
//!
//! What the model reads is the product. Section order, the exact strings and
//! the nudge lifetimes are behaviour, not presentation: "improving" a sentence
//! here changes the agent, and the compression machinery must stay invisible -
//! the injected text teaches retrieval vocabulary, never the mechanism.

pub mod Engine;
pub mod FirstTurn;
pub mod Hash;
pub mod Nudge;
pub mod Prompt;
pub mod Retrieval;
pub mod Section;
pub mod Signature;
pub mod TurnContext;
pub mod Window;

#[cfg(test)]
mod Test;

pub use FirstTurn::Fn::BuildFirstTurnInjection;
pub use Nudge::{Purge::Fn::PurgeExpiredNudges, Push::Fn::PushNudge, Render::Fn::RenderNudges};
pub use Prompt::{
	Const::{FallbackPrompts, ShippedSessionInject},
	Render::Fn::RenderPrompt,
};
pub use Signature::{Error::Fn::ErrorSig, Normalize::Fn::NormalizeArgsSig};
pub use TurnContext::BuildTurnContext;
pub use Window::{Stats::Struct::WindowStats, Turn::Fn::TurnWindow};
pub(crate) use Engine::EngineOffloadBlock;
pub(crate) use Retrieval::{AutoExpandGuidanceBlock, RetrievalGuidanceBlock};
pub(crate) use Section::{JoinSections, NudgesPresent, SectionLabel};
