//! The referenced-files domain's own fields.
//!
//! Split out of `AphroditeState` so the file-tracking hooks take
//! `&mut File::Struct`: recording a reference needs the recency list, and the
//! observation hook needs the tool set that decides what counts as a file
//! touch. Neither needs the store, the markers or the configuration.

use std::collections::VecDeque;

/// The referenced-files list, the tool set that feeds it, and the delta counter
/// the catalog renderer uses to avoid re-listing the same paths.
#[derive(Clone)]
pub struct Struct {
	/// Referenced files: {filepath: last_tool_name}
	pub ReferencedFiles:VecDeque<(String, String)>,
	/// File tools set.
	pub FileTools:Vec<String>,
	/// Number of referenced files the last time catalog_summary rendered,
	/// for delta-only file listing (04-F4: stops re-listing same 5 files).
	pub LastEmittedFileCount:usize,
}
