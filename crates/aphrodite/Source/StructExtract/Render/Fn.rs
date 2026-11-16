//! Render a parsed declaration as one capped signature line.

use crate::StructExtract::{
	Declaration::Enum::Enum as Declaration,
	Format::{Cap::Fn::Fn as Cap, Parameters::Fn as ParameterList, Signature::Fn as Signature},
};

/// Render a declaration, or `None` when it is not one.
///
/// Two shapes are rendered: a callable - kind, name, capped parameter list and optional return type -
/// and a plain named declaration, which is kind and name only. Both end up measured by the one line cap
/// in `Format::Cap`, and the callable form is capped AFTER the parameter list is capped, which is the
/// order the previous extractors used and therefore the width the model has always seen.
///
/// It is not a formatter: nothing is normalised, so a signature that wrapped across lines renders as
/// its first half. A `Malformed` declaration renders nothing at all rather than a `?` placeholder. The
/// declaration it is handed borrows the source line, so the rendered line is the only allocation.
pub(crate) fn Fn(Declaration:Declaration<'_>) -> Option<String> {
	let (Kind, Name, Parameters, Return) = match Declaration {
		Declaration::Declaration { Kind, Name, Parameters, Return } => (Kind, Name, Parameters, Return),
		Declaration::NotDeclaration | Declaration::Malformed { .. } => return None,
	};

	match Parameters {
		Some(Parameters) => {
			let Return = Return.map(|Return| format!(" -> {Return}")).unwrap_or_default();

			Some(Cap(format!("{Kind} {Name}({}){Return}", ParameterList(Parameters))))
		},
		None => Some(Signature(Kind, Name)),
	}
}
