//! The arrival model: the ETA heuristic scales with the bytes still
//! outstanding and refuses to extrapolate from nothing.

use crate::Prefetch::Arrival::Struct::Struct as Arrival;

#[test]
fn Eta() {
	// Half arrived after 10s -> ~10s more at the observed rate.
	let Eta = Arrival::EtaEstimate(50, 100, 10.0);
	assert!((Eta - 10.0).abs() < 0.001, "half arrived -> eta ~ elapsed, got {Eta}");
	// 90% arrived -> 10x less outstanding.
	let Eta2 = Arrival::EtaEstimate(90, 100, 10.0);
	assert!((Eta2 - 10.0 / 9.0).abs() < 0.001, "90% arrived -> ~1.11s, got {Eta2}");
	// Nothing arrived / fully arrived -> no ETA to extrapolate.
	assert_eq!(Arrival::EtaEstimate(0, 100, 10.0), 0.0);
	assert_eq!(Arrival::EtaEstimate(100, 100, 10.0), 0.0);
}
