//! Assembly of the tool table from its families.
//!
//! Every family contributes exactly one `(name, handler)` pair, so this module is
//! a list and nothing else: a new tool is a new leaf plus one line here, and the
//! schema/registry consistency test fails if only one of the two lands.

use std::collections::HashMap;

use super::{
	Catalog,
	Compress,
	ContextEngine,
	Diff,
	Directive,
	File,
	Prefetch,
	PrefetchStatus,
	Rebuild,
	Reclassify,
	Retrieve,
	Search,
	Stats,
	Test,
};
use crate::Tool::Dispatch::Fn::ToolHandler;
#[cfg(debug_assertions)]
use super::Debug;

/// Build the name -> handler table for every registered tool.
///
/// The dev-only `aphrodite_debug` family is added under `debug_assertions` only: a
/// release dylib that registered it fails `hermes plugins validate` rule 6 as an
/// undeclared tool, because the manifest declares exactly 13.
///
/// `context_engine_pre_llm` is in this table despite not being a model-visible
/// tool - it is an internal hook handler registered via
/// `ctx.register_context_engine`, and the schema/registry consistency test
/// excludes it by name for that reason.
pub(crate) fn Fn() -> HashMap<&'static str, ToolHandler> {
	let mut m:HashMap<&'static str, ToolHandler> = HashMap::new();

	[
		Compress::Fn::Fn(),
		Retrieve::Fn::Fn(),
		Stats::Fn::Fn(),
		Catalog::Fn::Fn(),
		Search::Fn::Fn(),
		Diff::Fn::Fn(),
		Directive::Fn::Fn(),
		File::Fn::Fn(),
		Prefetch::Fn::Fn(),
		PrefetchStatus::Fn::Fn(),
		Reclassify::Fn::Fn(),
		Test::Fn::Fn(),
		Rebuild::Fn::Fn(),
		ContextEngine::Fn::Fn(),
	]
	.into_iter()
	.for_each(|(name, handler)| {
		m.insert(name, handler);
	});

	#[cfg(debug_assertions)]
	{
		let (name, handler) = Debug::Fn::Fn();
		m.insert(name, handler);
	}

	m
}
