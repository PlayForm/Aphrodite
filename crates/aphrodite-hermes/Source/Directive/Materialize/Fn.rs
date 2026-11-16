//! Non-destructive write of the embedded directive set into one directory.
//!
//! The only side-effecting half of provisioning. Its contract is that it can
//! never lose user data and can never fail the caller: a missing file is
//! written, a byte-identical file is skipped silently, a DIFFERENT existing
//! file is skipped with a warning, and every I/O error becomes a warning in
//! the returned report rather than an error status. Callers therefore treat
//! `status: "ok"` as "the attempt completed", not "the files are correct".
//!
//! Layout: `Write/` decides one file's outcome, `Outcome/` is that decision as a
//! value, `Report/` is what the outcomes accumulate into, and this leaf folds
//! them. It owns no filesystem decision of its own.

use std::fs;

use crate::Directive::{
	Builtin,
	Entry,
	Materialize::{Outcome, Report, Write},
};

/// Provision the embedded builtin directives into `Dir`.
///
/// Idempotent and non-destructive: missing files are written, byte-identical
/// files are skipped, and existing DIFFERENT files are never overwritten
/// (skipped with a warning). Always returns a report with `status: "ok"` - every
/// failure degrades to a `warning`, never a panic or an error status.
///
/// It does not verify the result: a file that exists with the wrong bytes is
/// left alone by design, so a caller that needs the embedded text must read the
/// returned `warnings` instead of assuming the write won.
pub(crate) fn Fn(Dir:&std::path::Path) -> Report::Struct::Struct {
	let mut Report = Report::Struct::Struct {
		Dir:Dir.to_path_buf(),
		Written:Vec::new(),
		Skipped:Vec::new(),
		Warnings:Vec::new(),
	};

	// The directory is the precondition for every write, so the result of creating
	// it IS the decision: `Ok` folds the embedded set in, `Err` reports and stops.
	match fs::create_dir_all(Dir)
		.map_err(|e| format!("could not create directives dir {}: {}; nothing materialized", Dir.display(), e))
	{
		Ok(()) => Builtin::Fn::Fn().into_iter().for_each(|Entry| Fill(&mut Report, Entry)),
		Err(Message) => Fail(&mut Report, Message),
	}

	Report.Written.sort();
	Report.Skipped.sort();

	Report
}

/// Fold one embedded directive's outcome into the report.
fn Fill(Report:&mut Report::Struct::Struct, Entry:Entry::Struct::Struct) {
	match Write::Fn::Fn(&Report.Dir, &Entry) {
		Outcome::Enum::Enum::Written(File) => Report.Written.push(File),
		Outcome::Enum::Enum::Skipped(File) => Report.Skipped.push(File),
		Outcome::Enum::Enum::Warned { File, Message } => {
			crate::Debug::Warn::Fn::Fn(&Message);
			Report.Warnings.push(Message);
			Report.Skipped.push(File);
		},
	}
}

/// Nothing was materialized: say why, once, in both the log and the report.
fn Fail(Report:&mut Report::Struct::Struct, Message:String) {
	crate::Debug::Warn::Fn::Fn(&Message);

	Report.Warnings.push(Message);
}
