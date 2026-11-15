//! Write-path tests: the files land, a re-run writes nothing, and a
//! user-modified file is never clobbered.
//!
//! These three properties are the whole safety contract of provisioning, and
//! they are tested against real files rather than a mocked filesystem - the
//! interesting failure (a write that silently truncates user data) only exists
//! on a real one.

use std::fs;

use super::{Expected, Temp};
use crate::Directive::Materialize;

#[test]
fn materialize_writes_all_embedded_files() {
	let home = Temp::Fn::Fn("write");
	let dir = home.join("directives");
	let expected = Expected::Fn::Fn();

	let v = Materialize::Fn::Fn(&dir);
	assert_eq!(v["status"], "ok");
	assert_eq!(v["written"].as_array().unwrap().len(), expected.len());
	assert_eq!(v["skipped"].as_array().unwrap().len(), 0);
	assert_eq!(v["warnings"].as_array().unwrap().len(), 0);

	expected.iter().for_each(|(file_name, content)| {
		let p = dir.join(file_name);
		assert!(p.is_file(), "{file_name} must be materialized");
		assert_eq!(
			fs::read(&p).unwrap(),
			content.as_bytes(),
			"{file_name} must match the embedded bytes"
		);
	});

	fs::remove_dir_all(&home).ok();
}

#[test]
fn materialize_is_idempotent() {
	let home = Temp::Fn::Fn("idem");
	let dir = home.join("directives");
	let expected = Expected::Fn::Fn();

	let first = Materialize::Fn::Fn(&dir);
	assert_eq!(first["written"].as_array().unwrap().len(), expected.len());

	let second = Materialize::Fn::Fn(&dir);
	assert_eq!(second["written"].as_array().unwrap().len(), 0, "re-run must write nothing");
	assert_eq!(second["skipped"].as_array().unwrap().len(), expected.len());
	assert_eq!(second["warnings"].as_array().unwrap().len(), 0);

	expected.iter().for_each(|(file_name, content)| {
		assert_eq!(fs::read(dir.join(file_name)).unwrap(), content.as_bytes());
	});

	fs::remove_dir_all(&home).ok();
}

#[test]
fn materialize_never_overwrites_user_modified_file() {
	let home = Temp::Fn::Fn("keep");
	let dir = home.join("directives");
	fs::create_dir_all(&dir).unwrap();
	let expected = Expected::Fn::Fn();
	let victim = dir.join("focus.md");
	let user_content = b"# focus\nUSER-CUSTOMIZED-DIRECTIVE\n";
	fs::write(&victim, user_content).unwrap();

	let v = Materialize::Fn::Fn(&dir);
	assert_eq!(v["status"], "ok");
	assert!(
		v["skipped"].as_array().unwrap().iter().any(|s| s == "focus.md"),
		"focus.md must be reported as skipped: {v}"
	);
	assert_eq!(v["written"].as_array().unwrap().len(), expected.len() - 1);
	assert!(
		v["warnings"]
			.as_array()
			.unwrap()
			.iter()
			.any(|w| w.as_str().unwrap().contains("user-modified")),
		"a user-modified skip must carry a warning: {v}"
	);
	assert_eq!(
		fs::read(&victim).unwrap(),
		user_content,
		"user-modified file must be left untouched"
	);

	expected.iter().for_each(|(file_name, content)| {
		match file_name.as_str() {
			"focus.md" => {},
			_ => assert_eq!(fs::read(dir.join(file_name)).unwrap(), content.as_bytes()),
		}
	});

	fs::remove_dir_all(&home).ok();
}
