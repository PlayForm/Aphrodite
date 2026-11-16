//! `context_engine_pre_llm` - the context-engine pre-LLM hook handler.
//!
//! 05-P1/T1: the same single assembler as the bridge `pre_llm_call` arm and core
//! `Hook::pre_llm_call`, so this path cannot fork on what the model sees.
//!
//! Not a model-visible tool: it is registered through
//! `ctx.register_context_engine`, which is why it has no schema and is excluded by
//! name from the schema/registry consistency test.

use crate::Tool::Dispatch::Fn::Handler;

/// The `context_engine_pre_llm` handler.
///
/// Returns `null` when the assembler produced nothing, so the engine injects no
/// empty context block.
pub(crate) fn Fn() -> (&'static str, Handler) {
	("context_engine_pre_llm", |_Args| {
		crate::Session::With::Fn::with_shared(|State| {
			let Context = aphrodite::Flow::TurnContext::Fn::Fn(State, None);

			match Context.is_empty() {
				true => serde_json::Value::Null,
				false => serde_json::json!({ "context": Context }),
			}
		})
	})
}
