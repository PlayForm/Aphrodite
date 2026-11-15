//! `IsoTimestamp`: the UTC record stamp.

/// Readable UTC timestamp `YYYY-MM-DDTHH:MM:SS.ffffff` (no chrono dep - this
/// is dev-only instrumentation; a hand-rolled civil-from-days conversion
/// keeps the dependency tree untouched).
///
/// Always UTC and always `Z`, so two logs from two machines line up without a
/// timezone in the record. The conversion is Howard Hinnant's civil-from-days;
/// it is exact for every date the proleptic Gregorian calendar can express, and
/// it cannot fail - a clock before the epoch degrades to zero rather than
/// panicking inside a logger.
///
/// The three ternaries of the original algorithm are `then_some`/`unwrap_or`
/// pairs. Both arms of each are arithmetic on integers already in hand, so
/// nothing is evaluated eagerly that the branch would have skipped, and the
/// `then_some` spelling states the same three choices the paper states.
pub(crate) fn Fn() -> String {
	let Now = std::time::SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.unwrap_or_default();
	let Secs = Now.as_secs();
	let Micros = Now.subsec_micros();
	let Days = Secs / 86_400;
	let Rem = Secs % 86_400;
	let (Hh, Mm, Ss) = (Rem / 3_600, (Rem % 3_600) / 60, Rem % 60);
	// Howard Hinnant's civil_from_days: days -> (y, m, d) in the proleptic
	// Gregorian calendar.
	let z = Days as i64 + 719_468;
	// Hinnant writes the era as `(z >= 0 ? z : z - 146096) / 146097`, which is
	// floor division by 146097 spelled with a truncating operator; `div_euclid`
	// is that division directly, and the two agree for every `z`.
	let Era = z.div_euclid(146_097);
	let Doe = z - Era * 146_097; // [0, 146096]
	let Yoe = (Doe - Doe / 1_460 + Doe / 36_524 - Doe / 146_096) / 365; // [0, 399]
	let y = Yoe + Era * 400;
	let Doy = Doe - (365 * Yoe + Yoe / 4 - Yoe / 100); // [0, 365]
	let Mp = (5 * Doy + 2) / 153; // [0, 11]
	let d = (Doy - (153 * Mp + 2) / 5 + 1) as u32; // [1, 31]
	// `m` is `Mp + 3` for the first ten months and `Mp - 9` for the last two,
	// which is the same wrap as `(Mp + 2) % 12 + 1` over `Mp`'s range [0, 11].
	let m = ((Mp + 2) % 12 + 1) as u32; // [1, 12]
	// January and February belong to the previous Gregorian year.
	let y = (y + (m <= 2) as i64) as u32;

	format!("{y:04}-{m:02}-{d:02}T{Hh:02}:{Mm:02}:{Ss:02}.{Micros:06}Z")
}
