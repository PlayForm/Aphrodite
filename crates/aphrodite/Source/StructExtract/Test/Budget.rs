//! The preview budget: a file with far more declarations than fit is capped rather
//! than flooding the marker the model reads.

use crate::StructExtract::Extract::Fn as ExtractCodeStructure;

#[test]
fn TestBudgetRespected() {
	// Generate lots of functions to test budget
	let mut Code = String::new();
	(0..50).for_each(|i| {
		Code.push_str(&format!("fn func{}(x: i32, y: i32, z: i32) -> i32 {{ 42 }}\n", i));
	});
	let r = ExtractCodeStructure(&Code, "rust");
	// Should have stopped before 50 due to budget
	match r.get("fns") {
		Some(Fns) => {
			assert!(Fns.len() < 50, "budget should cap output: got {}", Fns.len());
		},
		None => {},
	}
}
