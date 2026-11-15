//! `Decide`: the retrieval ratio and the bounds as one of three movements.

use super::super::Enum::Enum as Trend;

/// Decide which way the threshold moves, given the ratio and the bounds.
///
/// Two thresholds, not a gradient: at or above 50% retrieval the agent is using
/// the segments, so split more eagerly; below 25% it is ignoring them, so only
/// split long chains. The dead band between the two is deliberate.
///
/// The bounds are part of the decision, not a second check at the consumer: a
/// ratio that says "decrease" while the threshold already sits on its floor is
/// `Hold`, and that is one value rather than a `if` the caller has to remember.
/// `then_some`/`or_else` keep both tests lazy - the second ratio test does not
/// run once the first has decided, which is the same short circuit the two
/// ordered match guards produced.
pub(crate) fn Fn(Ratio:f64, Current:usize, Floor:usize, Maximum:usize) -> Trend {
	(Ratio >= 0.5 && Current > Floor)
		.then_some(Trend::Decrease)
		.or_else(|| (Ratio < 0.25 && Current < Maximum).then_some(Trend::Increase))
		.unwrap_or(Trend::Hold)
}
