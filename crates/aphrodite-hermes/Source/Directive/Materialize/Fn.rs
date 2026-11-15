//! Non-destructive write of the embedded directive set into one directory.
//!
//! The only side-effecting half of provisioning. Its contract is that it can
//! never lose user data and can never fail the caller: a missing file is
//! written, a byte-identical file is skipped silently, a DIFFERENT existing
//! file is skipped with a warning, and every I/O error becomes a warning in
//! the returned report rather than an error status. Callers therefore treat
//! `status: "ok"` as "the attempt completed", not "the files are correct".

use std::{fs, path::Path};

use serde_json::json;

use crate::Directive::Builtin;

/// Provision the embedded builtin directives into `dir`.
///
/// Idempotent and non-destructive: missing files are written, byte-identical
/// files are skipped, and existing DIFFERENT files are never overwritten
/// (skipped with a warning). Always returns `status: "ok"` - every failure
/// degrades to a `warning`, never a panic or an error status.
///
/// It does not verify the result: a file that exists with the wrong bytes is
/// left alone by design, so a caller that needs the embedded text must read
/// the returned `warnings` instead of assuming the write won.
pub(crate) fn Fn(dir:&Path) -> serde_json::Value {
	let mut written:Vec<String> = Vec::new();
	let mut skipped:Vec<String> = Vec::new();
	let mut warnings:Vec<String> = Vec::new();

	match fs::create_dir_all(dir) {
		Err(e) => {
			let msg = format!("could not create directives dir {}: {}; nothing materialized", dir.display(), e);
			warnings.push(msg.clone());
			crate::Debug::Warn::Fn::Fn(&msg);

			return json!({
				"status": "ok",
				"dir": dir.to_string_lossy(),
				"written": written,
				"skipped": skipped,
				"warnings": warnings,
			});
		},
		Ok(()) => {},
	}

	Builtin::Fn::Fn().into_iter().for_each(|(name, content)| {
		let file_name = format!("{name}.md");
		let dest = dir.join(&file_name);

		match fs::read(&dest) {
			Ok(existing) if existing == content.as_bytes() => skipped.push(file_name),
			Ok(_) => {
				warnings.push(format!(
					"directives/{file_name} exists with different content; leaving user-modified file as-is (not \
					 overwritten)"
				));
				skipped.push(file_name);
			},
			Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
				match fs::write(&dest, content.as_bytes()) {
					Ok(()) => written.push(file_name),
					Err(we) => {
						warnings.push(format!("could not write directives/{file_name}: {we}"));
						crate::Debug::Warn::Fn::Fn(&format!("directives/{file_name} write failed: {we}"));
					},
				}
			},
			Err(e) => {
				warnings.push(format!("could not read directives/{file_name}: {e}; leaving as-is"));
				skipped.push(file_name);
			},
		}
	});

	written.sort();
	skipped.sort();

	json!({
		"status": "ok",
		"dir": dir.to_string_lossy(),
		"written": written,
		"skipped": skipped,
		"warnings": warnings,
	})
}
