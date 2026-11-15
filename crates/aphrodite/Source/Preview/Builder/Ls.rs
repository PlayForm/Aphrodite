//! Directory-listing preview arm.
//!
//! A listing is described by how many entries it has and which extensions
//! dominate. This arm renders that. It is not a filesystem query - nothing is
//! stat'ed, so a listing of files that no longer exist previews the same way.

use crate::Preview::Input::Struct::Struct as Input;

/// Build a preview of a directory listing.
///
/// Promises file and directory counts plus the most common extensions. It is not
/// a tree: nesting is flattened, so a listing of one directory of directories
/// and a deep tree both report a file count and nothing about depth.
pub(crate) fn Fn(Inp:&Input<'_>) -> String {
	use std::collections::HashMap;
	let mut Files = 0usize;
	let mut dirs = 0usize;
	let mut Ext:HashMap<String, usize> = HashMap::new();

	Inp.raw
		.lines()
		.filter(|Line| {
			let t = Line.trim();

			// Skip the `total N` header `ls -l` prints (not a filesystem entry).
			match t.strip_prefix("total ") {
				Some(Rest) => Rest.is_empty() || !Rest.chars().all(|c| c.is_ascii_digit()),
				None => !t.is_empty(),
			}
		})
		.for_each(|Line| {
			let t = Line.trim();
			let b = Line.as_bytes();
			// `ls -l` long form: mode string in the first column.
			let IsLong = b.len() >= 10
				&& matches!(b[0], b'-' | b'd' | b'l' | b'c' | b'b' | b'p' | b's')
				&& b[1..10]
					.iter()
					.all(|&c| matches!(c, b'r' | b'w' | b'x' | b'-' | b's' | b't' | b'S' | b'T'));
			let (IsDirectory, name) = match IsLong {
				true => {
					let name = Line.split_whitespace().last().unwrap_or("");

					(b[0] == b'd', name)
				},
				false => {
					match t.strip_suffix('/') {
						Some(Stripped) => (true, Stripped),
						None => (false, t),
					}
				},
			};

			match IsDirectory {
				true => {
					dirs += 1;
				},
				false => {
					Files += 1;
					// File extension: text after the last `.` in the basename.
					let Base = name.rsplit('/').next().unwrap_or(name);
					let Dot = Base.rfind('.').filter(|&Dot| Dot > 0 && Dot < Base.len() - 1);

					match Dot {
						Some(Dot) => {
							let e:String = Base[Dot..].chars().take(8).collect();

							*Ext.entry(e).or_insert(0) += 1;
						},
						None => {},
					}
				},
			}
		});

	match Files == 0 && dirs == 0 {
		true => {
			return format!("[ls:{}L]", Inp.Total);
		},
		false => {},
	}

	let mut Top:Vec<(String, usize)> = Ext.into_iter().collect();
	Top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
	let ExtStr = Top
		.iter()
		.take(3)
		.map(|(e, n)| format!("{}×{}", e, n))
		.collect::<Vec<_>>()
		.join(" ");

	match ExtStr.is_empty() {
		true => format!("[ls:{} files {} dirs]", Files, dirs),
		false => format!("[ls:{} files {} dirs | {}]", Files, dirs, ExtStr),
	}
}
