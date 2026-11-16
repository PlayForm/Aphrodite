# The state domains

`AphroditeState` was one struct with 51 fields across nine concerns. It is now a composition of nine
per-domain sub-structs, each in the directory that owns its operations. This note records the map and
the evidence it was derived from, so a later change can tell a decision from a drift.

---

## Why the struct had to split

The naming doctrine says the path is the namespace and the name is the word. A **field** has no path
segment of its own - its path is its struct - so `RecentMarkers` could not become `Recent` while
`TurnCounter` became `Counter` in the same struct: one word each would have collapsed nine domains'
overlapping words onto the same names.

`NAMING-VERDICT.md` §`B6` recorded exactly this as the residue a rename could not reach: "a structural
rewrite, not a rename". This is that rewrite.

The second reason is authority. `&mut AphroditeState` was the capability every function took, so a
function that recorded one marker could also clear the store, advance the turn and rewrite the
configuration - and a reader had to check it did not. A domain sub-struct is a narrower capability the
compiler enforces.

---

## The map

The domains were derived from **usage**, not from the current field names: which functions read and
write which fields together. The reader sets are disjoint for the first seven, which is what makes them
separate domains rather than one.

- **`Store`** - `Map`, `Order`, `Bytes`, `Budget`. Read and written only by `State/Store/*`; every other
  reader goes through `Get`/`Contains`.
- **`Marker`** - `Recent`, `Emitted`. Read by `Catalog/*`, `Hook/LLM/*`, `Prefetch/*`, `Session/Catalog/*`,
  `ABI/*`, the Hermes tool registry.
- **`Event`** - `Recent`. Read by `Hook/Meta`, `Hook/*/Entry/Meta`, `Flow/Window/*`, `Session/Start`.
  Disjoint from the marker readers - one ring is markers, one is tool calls.
- **`File`** - `Recent`, `Tools`, `Emitted`. Read by `Hook/Path/*`, `Hook/Observation`, `Session/Catalog`.
- **`Turn`** - `Counter`, `Scanned`, `Index`. The hottest domain: 104 counter reads, 52 index reads.
- **`ChainSplit`** - `Enable`, `Minimum`, `Floor`, `Maximum`, `Ledger`, `Segments`, `Next`. Owned by the
  Tier 1 teaching loop; only `ConfigurationLoader` writes the bounds.
- **`Directive`** - `Loaded`, `Active`, `Ephemeral`, `Manual`. Owned by `Directive/Verb/*` and the
  turn-context assembler.
- **`PollWorker`** - `Tasks`, `Enable`. 104 of the 106 `Tasks` reads are inside `PollWorker/`.
- **`Configuration`** - the 25 values mirrored out of `aphrodite.toml`. Written once by
  `ConfigurationLoader::Configuration::Apply`, read one key at a time by everyone else.

Six was the working guess. The usage graph says nine: `Turn`, `PollWorker` and `Configuration` have their
own reader sets and their own writers, and folding any of them into a neighbour would have put two
writers on one struct.

---

## The qualifiers that were dropped

Each domain's fields are one word, because the domain segment now carries the qualifier.

- **Store** - `InlineStore`→`Map`, `InlineOrder`→`Order`, `InlineStoreBytes`→`Bytes`,
  `InlineStoreByteBudget`→`Budget`.
- **Marker** - `RecentMarkers`→`Recent`, `LastEmittedMarkerCount`→`Emitted`.
- **Event** - `ToolEvents`→`Recent`. Three domains call their ring `Recent`; the path tells them apart,
  and the shared word is the point - "the recent ring of X" reads the same way everywhere.
- **File** - `ReferencedFiles`→`Recent`, `FileTools`→`Tools`, `LastEmittedFileCount`→`Emitted`.
- **Turn** - `TurnCounter`→`Counter`, `ScannedMessageIndex`→`Scanned`, `ConvIndex`→`Index`.
- **ChainSplit** - `ChainSplitEnabled`→`Enable`, `ChainSplitMinimumSegments`→`Minimum`,
  `ChainSplitFloor`→`Floor`, `ChainSplitMaximumSegments`→`Maximum`, `SplitEvents`→`Ledger`,
  `SplitSegmentMap`→`Segments`, `SplitNextEventID`→`Next`.
- **Directive** - `Directives`→`Loaded`, `ActiveDirectives`→`Active`, `EphemeralDirectives`→`Ephemeral`,
  `ManualDirectiveTurn`→`Manual`.
- **PollWorker** - `BgTasks`→`Tasks`, `PollWorkerEnabled`→`Enable`.
- **Configuration** - `APIURL`→`URL`, `ConfigurationError`→`Error`. Nothing else drops.

---

## What resisted

`Configuration`'s remaining 23 names keep their subsystem qualifier - `ToolThreshold` beside
`TerminalThreshold` beside `EngineThresholdPct`, `EngineProtectFirst` beside `EngineProtectLast`. The
path `State::Configuration` says _that_ these are configuration, not _which_ subsystem each knob
belongs to, so stripping the qualifier would collapse three thresholds onto one word inside one struct.
That is the collision rule in `RUST-NAMING-DOCTRINE.md` §4, and it is the same residue `B6` named.

Splitting `Configuration` by its TOML section was considered and rejected: no function reads a section as
a unit, they read one key, so a section split would be grouping by source file rather than by usage -
the thing the card said not to do.

`ChainSplit::Minimum` sits near `ChainSplit::Window::Const::Minimum` (the adaptation window's evidence
floor). Different scopes, different meanings: the field is the segment-count threshold, the constant is
how many events the ratio needs. The field's doc comment says which.

---

## The signatures that narrowed

Every `impl AphroditeState` block is gone. The state's whole operation surface now takes the domain it
touches:

- **`Store::Struct`** - `Put`, `Get`, `Contains`, `Set`, `Evict`, plus the two free helpers
  `OverBudget(&Store::Struct)` and `Step(&mut Store::Struct)`. `Put` alone had 79 call sites.
- **`Marker::Struct`** - `Record` (was `RecordMarker`, 22 call sites).
- **`Event::Struct`** - `Record` (was `RecordEvent`, 5).
- **`File::Struct`** - `Record` (was `RecordFile`, 6).
- **`ChainSplit::Struct`** - `Record` (10), `Retrieve` (9), `Adapt`. `Record` takes the turn number as an
  argument instead of reading `State.Turn.Counter`, so the loop has no path to the turn bookkeeping.
- **`Session::Archive::Fn`**, **`Session::Turn::Fn`**, **`Session::Index::Fn`** - take
  `&mut Turn::Struct` / `&Turn::Struct`.

The 119 functions still taking `&AphroditeState` are the hooks, the flow assembler and the catalog build,
each of which genuinely reads three or more domains. Narrowing those means composing two borrows at the
call site; that is a separate pass, and doing it here would have made one card both the structure move
and the caller redesign.

---

## Claim-to-test

| Claim                                       | Probe                                                                    | Pass condition                                          |
| ------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------- |
| Every field resolves through a domain path  | `python3 .hermes/scripts/state_verify.py` (prints fields per domain)     | 51 fields across 9 domains, 0 unknown names             |
| No field repeats its path's qualifier       | each domain's `Struct.rs`; the field's words are not the domain's word   | no `Store::Struct::InlineStore`                         |
| No state field is accessed without its path | grep the old field names across `crates/`                                | only English prose and other structs' same-named fields |
| The seed is unchanged                       | the nine `<Domain>/Default.rs` files vs the pre-split `State/Default.rs` | byte-identical values                                   |
| No `impl AphroditeState` remains            | `grep -rn 'impl AphroditeState' crates --include=*.rs`                   | no output                                               |
| Formatting is rustfmt-clean                 | `cargo fmt --all -- --check`                                             | exit 0                                                  |
| Both crates still build                     | `cargo check -p aphrodite -p aphrodite-hermes --all-targets`             | exit 0 (verification card)                              |
