//! Poll ingestion, split by what it proves: `Poll/` covers exit-code
//! detection and output accumulation, `Live/` covers the live-container
//! progress surface and the instant-marker contract.

mod Live;
mod Poll;
