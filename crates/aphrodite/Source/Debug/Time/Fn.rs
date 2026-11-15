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
	let Era = if z >= 0 { z } else { z - 146_096 } / 146_097;
	let Doe = z - Era * 146_097; // [0, 146096]
	let Yoe = (Doe - Doe / 1_460 + Doe / 36_524 - Doe / 146_096) / 365; // [0, 399]
	let y = Yoe + Era * 400;
	let Doy = Doe - (365 * Yoe + Yoe / 4 - Yoe / 100); // [0, 365]
	let Mp = (5 * Doy + 2) / 153; // [0, 11]
	let d = (Doy - (153 * Mp + 2) / 5 + 1) as u32; // [1, 31]
	let m = if Mp < 10 { Mp + 3 } else { Mp - 9 } as u32; // [1, 12]
	let y = (if m <= 2 { y + 1 } else { y }) as u32;
	format!("{y:04}-{m:02}-{d:02}T{Hh:02}:{Mm:02}:{Ss:02}.{Micros:06}Z")
}
