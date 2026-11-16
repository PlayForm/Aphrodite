//! The Python extractor: `def` lines become `Fns` and `class` lines become
//! `Classes`, both out of the same preview budget.

use crate::StructExtract::Extract::Fn as Structure;

#[test]
fn TestExtractPython() {
	let Code = "def hello(name: str) -> str:\n    return name\n\nclass MyClass:\n    pass\n";
	let r = Structure(Code, "python");
	assert!(r.contains_key("fns"));
	assert!(r.contains_key("classes"));
}
