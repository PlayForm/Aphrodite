//! The Rust extractor: functions with their return types, structs, and the
//! `pub(crate) async fn` form that used to be invisible to the prefix check.

use crate::StructExtract::Extract::Fn as ExtractCodeStructure;

#[test]
fn TestExtractRustFns() {
	let Code = "pub fn main() -> i32 {\n    42\n}\nfn helper(x: i32) -> bool {\n    true\n}\n";
	let r = ExtractCodeStructure(Code, "rust");
	assert!(r.contains_key("fns"));
	let Fns = &r["fns"];
	assert!(Fns.iter().any(|s| s.contains("main")));
	assert!(Fns.iter().any(|s| s.contains("helper")));
}

#[test]
fn TestExtractRustStructs() {
	let Code = "pub struct Foo {\n    x: i32,\n}\nstruct Bar<T> {}\n";
	let r = ExtractCodeStructure(Code, "rust");
	assert!(r.contains_key("structs"));
}

/// `pub(crate) async fn` was previously invisible to the Rust extractor's
/// `IsFn` prefix check (only the non-async `pub(crate) fn` was listed).
#[test]
fn TestExtractRustPubCrateAsyncFn() {
	let Code = "pub(crate) async fn tick(&self) -> bool {\n    true\n}\n";
	let r = ExtractCodeStructure(Code, "rust");
	let Fns = r.get("fns").expect("pub(crate) async fn should be extracted");
	assert!(Fns.iter().any(|s| s.contains("tick")), "expected tick() in {Fns:?}");
}
