//! One member of the exposition.

use crate::Boot::Route::Metrics::Entry;

/// One member of the exposition, in the order it is written.
#[derive(Clone, Copy)]
pub enum Enum {
	/// One counter line, or no line when the stats document omits its member.
	Entry(Entry::Struct::Struct),
	/// The latency histogram: five buckets, a count and a sum.
	Histogram,
}
