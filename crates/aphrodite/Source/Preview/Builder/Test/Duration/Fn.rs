//! The run duration, when the runner printed one.

/// First duration token on a line (`0.31s` / `150ms`), first match wins.
///
/// A whitespace token ending `ms` with an all-digit prefix, or ending `s` with a
/// `digits '.' digits` prefix. It is not a time parser: the token is returned as written, so
/// the preview shows the runner's own spelling of its duration.
pub(crate) fn Fn(Content:&str) -> Option<String> {
	Content.lines().flat_map(|Line| Line.split_whitespace()).find_map(|Token| {
		match Token.strip_suffix("ms") {
			Some(Milliseconds) => {
				(!Milliseconds.is_empty() && Milliseconds.chars().all(|Character| Character.is_ascii_digit()))
					.then(|| format!("{Milliseconds}ms"))
			},
			None => {
				Token.strip_suffix('s').and_then(|Seconds| {
					let (Whole, Fraction) = Seconds.split_once('.')?;
					let Numeric = !Whole.is_empty()
						&& !Fraction.is_empty()
						&& Whole.chars().all(|Character| Character.is_ascii_digit())
						&& Fraction.chars().all(|Character| Character.is_ascii_digit());

					Numeric.then(|| format!("{Seconds}s"))
				})
			},
		}
	})
}
