//! The samples the smoke test round-trips.

/// One sample per content family the compressor special-cases, in report order.
///
/// There is ONE list: `quick` runs the first entry and `full` runs all of them, so
/// the two modes cannot disagree about what the quick sample is. The previous
/// shape repeated the first sample in both arms of a `match`, which is a
/// divergence waiting for the next sample added to only one of them.
pub(crate) const SAMPLES:&[(&str, &str)] = &[
	("fn main() { println!(\"hi\"); }\n", "source_code"),
	("error[E0382]: borrow of moved value\nwarning: unused\n", "build"),
	("{\"a\":1,\"b\":2,\"c\":3}\n", "json_array"),
];
