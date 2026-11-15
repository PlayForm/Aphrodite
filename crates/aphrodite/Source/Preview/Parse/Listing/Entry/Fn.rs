//! Recognise one listing entry.

use crate::Preview::Parse::Listing::{Entry::Struct::Struct as Entry, Kind::Enum::Enum as Kind};

/// Longest mode string an `ls -l` line opens with.
const ModeLength:usize = 10;

/// Reduce a listing line to its name and kind.
///
/// Two shapes are read: the `ls -l` long form, recognised by a mode string in the first
/// column (whose first byte names the kind), and the bare form, where a trailing `/` is
/// the only directory signal available. It is not a filesystem query - nothing is
/// stat'ed - and it never fails: a line the caller decided to keep always becomes an
/// entry, because dropping it silently is how a listing preview starts under-counting.
pub(crate) fn Fn(Line:&str) -> Entry {
	let Trimmed = Line.trim();
	let Bytes = Line.as_bytes();
	let Long = Bytes.len() >= ModeLength
		&& matches!(Bytes[0], b'-' | b'd' | b'l' | b'c' | b'b' | b'p' | b's')
		&& Bytes[1..ModeLength]
			.iter()
			.all(|Byte| matches!(Byte, b'r' | b'w' | b'x' | b'-' | b's' | b't' | b'S' | b'T'));

	let (Kind, Name) = match Long {
		true => {
			(
				match Bytes[0] {
					b'd' => Kind::Directory,
					_ => Kind::File,
				},
				Line.split_whitespace().last().unwrap_or(""),
			)
		},
		false => {
			match Trimmed.strip_suffix('/') {
				Some(Stripped) => (Kind::Directory, Stripped),
				None => (Kind::File, Trimmed),
			}
		},
	};

	Entry { Name:Name.to_string(), Kind }
}
