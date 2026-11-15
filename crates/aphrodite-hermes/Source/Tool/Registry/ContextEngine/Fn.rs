//! `context_engine_pre_llm` - the context-engine pre-LLM hook handler.
//!
//! 05-P1/T1: the same single assembler as the bridge `pre_llm_call` arm and core
//! `Hook::pre_llm_call`, so this path cannot fork on what the model sees.
//!
//! Not a model-visible tool: it is registered through
//! `ctx.register_context_engine`, which is why it has no schema and is excluded by
//! name from the schema/registry consistency test.

use Tool::Dispatch::Fn::ToolHandler;

/// The `context_engine_pre_llm` handler.
///
/// Returns `null` when the assembler produced nothing, so the engine injects no
/// empty context block.
pub(crate) fn Fn() -> (&'static str, ToolHandler) {
	("context_engine_pre_llm", |_args| {
		crate::Session::With::Fn::with_shared(|state| {
			let context = aphrodite::Flow::BuildTurnContext(state, None);
			if context.is_empty() {
				serde_json::Value::Null
			} else {
				serde_json::json!({"context": context})
			}
		})
	})
}
