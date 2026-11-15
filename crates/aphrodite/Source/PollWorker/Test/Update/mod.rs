//! Poll ingestion, split by what it proves: `Poll/` covers exit-code detection and
//! output accumulation, `Live/` covers the live-container progress surface and the
//! instant-marker contract, `Attribution/` covers which task a poll applies to, and
//! `Cap/` covers the accumulator's bound.

mod Attribution;
mod Cap;
mod Live;
mod Poll;
