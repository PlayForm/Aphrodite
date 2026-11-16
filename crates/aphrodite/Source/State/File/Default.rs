//! The referenced-files seed: the default file-tool set and an empty list.

use std::collections::VecDeque;

use super::Struct::Struct;

impl Default for Struct {
	fn default() -> Self {
		Self {
			ReferencedFiles:VecDeque::new(),
			FileTools:vec![
				"read_file".into(),
				"write_file".into(),
				"patch".into(),
				"search_files".into(),
			],
			LastEmittedFileCount:0,
		}
	}
}
