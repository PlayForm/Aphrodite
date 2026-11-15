//! The category order per language.

use crate::StructExtract::{
	Policy::Category::Struct::Struct as Category,
	Recognize::{
		Go::{Function as GoFunction, Type as GoType},
		JS::{Arrow as JSArrow, Class as JSClass, Function as JSFunction},
		Python::{Class as PythonClass, Function as PythonFunction},
		Rust::{Function as RustFunction, Impl as RustImpl, Struct as RustStruct, Trait as RustTrait},
	},
};

/// Rust: functions, then structs, then traits, then impls.
static RUST:[Category; 4] = [
	Category { Key:"fns", Recognize:&[RustFunction::Fn::Fn] },
	Category { Key:"structs", Recognize:&[RustStruct::Fn::Fn] },
	Category { Key:"traits", Recognize:&[RustTrait::Fn::Fn] },
	Category { Key:"impls", Recognize:&[RustImpl::Fn::Fn] },
];

/// Python: functions, then classes.
static PYTHON:[Category; 2] = [
	Category { Key:"fns", Recognize:&[PythonFunction::Fn::Fn] },
	Category { Key:"classes", Recognize:&[PythonClass::Fn::Fn] },
];

/// Go: functions, then types.
static GO:[Category; 2] = [
	Category { Key:"fns", Recognize:&[GoFunction::Fn::Fn] },
	Category { Key:"types", Recognize:&[GoType::Fn::Fn] },
];

/// JavaScript and TypeScript: functions (declarations and arrow bindings), then classes.
static JS:[Category; 2] = [
	Category { Key:"fns", Recognize:&[JSFunction::Fn::Fn, JSArrow::Fn::Fn] },
	Category { Key:"classes", Recognize:&[JSClass::Fn::Fn] },
];

/// The ordered categories for a language tag, or an empty list.
///
/// The order IS the priority: the budget is spent category by category, so a file with many functions
/// reports no traits rather than a fraction of each. An unknown tag yields an empty list, which is a
/// preview with no structure - a degraded marker, not a failure.
///
/// It is not a dispatch table to the extractors: it names categories and recognisers, and the scanner
/// is what applies them.
pub(crate) fn Fn(Language:&str) -> &'static [Category] {
	match Language {
		"rust" => &RUST,
		"python" => &PYTHON,
		"go" => &GO,
		"js" | "ts" => &JS,
		_ => &[],
	}
}
