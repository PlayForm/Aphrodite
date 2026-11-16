//! The entry-point gates: size floor, unknown type, and "reduction must save".

use crate::Stage2::Compress::Fn::Fn as Compress;

#[test]
fn TestSmallContentReturnsNone() {
	assert_eq!(Compress("short", "code_rust"), None);
}

#[test]
fn TestUnknownTypeReturnsNone() {
	let Content = "a".repeat(100);
	assert_eq!(Compress(&Content, "unknown_type"), None);
}

#[test]
fn TestStage2SkipsSmall() {
	assert_eq!(Compress(r#"{"a":1}"#, "json"), None);
}

#[test]
fn TestIntegrationBuildStage2() {
	let Content = format!("   Compiling foo v0.1.0\n   error: something broke\n   {}", "x".repeat(80));
	let Result = Compress(&Content, "build_error");
	assert!(Result.is_some());
	assert!(Result.unwrap().len() < Content.len());
}
