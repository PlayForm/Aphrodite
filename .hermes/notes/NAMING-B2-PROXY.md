# Naming B2 - the proxy pipeline

One word per item across `crates/aphrodite/Source/Proxy/**`, under
`RUST-NAMING-DOCTRINE.md`. The path is the namespace; the identifier is the word.
Behaviour is unchanged: this is a rename, and every emitted name is verified
below.

## What was done

- 59 tracked files rewritten, 685 insertions / 678 deletions - the rename itself
  is a pure substitution (678/678); the seven extra lines are the doc comments
  §6 requires on the renamed items. Two files added
  (`Proxy/Request/Route/Chat/{mod,Const}.rs`) and one facade line added to
  `Proxy/Request/Route/mod.rs`.
- 155 identifier pairs applied through per-file rules: a rule is either an
  exact identifier token or an exact substring, and a rule that matches nothing
  is reported, so a bad rule cannot land silently. Cross-partition field
  accesses (`Compression.and_then(|Table| Table.Limit)`) were protected by
  anchored rules rather than by a global token replace, which is why the
  configuration fields another card renamed are untouched here.
- `rustfmt --edition 2024 --check` over every touched file: clean. That parse is
  also the syntax proof for this card - no build was run.

## The rename, by group

**`Proxy::State::Struct` (the `AppState` fields)**

| Was                      | Now               | Where the dropped words went                            |
| ------------------------ | ----------------- | ------------------------------------------------------- |
| `CacheCompressThreshold` | `CacheThreshold`  | `Compress` - every threshold here is compression's      |
| `TokenCompressThreshold` | `TokenThreshold`  | as above                                                |
| `InlineCCRThreshold`     | `InlineThreshold` | `CCR` - the inline store is the only CCR beside it      |
| `AutoExpandLimit`        | `ExpandLimit`     | `Auto` - the flag beside it is `AutoExpand`             |
| `MaximumRequestBodySize` | `BodyLimit`       | `Maximum`/`Size` - a cap is a maximum, `usize` is bytes |
| `CodeMultiplierX100`     | `CodeMultiplier`  | `X100` - fixed point is the `/ 100.0` at the read       |
| `CompressionRatioEma`    | `RatioEma`        | `Compression` - `State::Method::Ratio` names it         |
| `CompressionsByType`     | `Compressions`    | `ByType` - the map's key type is the doc's job          |
| `ResponseCacheTtl`       | `CacheTtl`        | `Response` - `ResponseCache` beside it carries it       |
| `RequestBodyBytes`       | `RequestBytes`    | `Body` - a byte count of a body is a byte count         |
| `ResponseBodyBytes`      | `ResponseBytes`   | as above                                                |
| `BodyBytesSaved`         | `BytesSaved`      | `Body` - it counts content bytes, see the doc           |
| `ToolRelayCalls`         | `RelayCalls`      | `Tool` - `ToolRelay` is the enable flag beside it       |
| `ToolRelaySuccess`       | `RelaySuccess`    | as above                                                |
| `ToolRelayFailure`       | `RelayFailure`    | as above                                                |
| `UpstreamConnectErrors`  | `ConnectErrors`   | `Upstream` - the error family is documented             |
| `UpstreamOtherErrors`    | `OtherErrors`     | as above                                                |
| `UpstreamLatencyMicros`  | `UpstreamLatency` | `Micros` - the unit is in the doc comment               |
| `TotalLatencyMicros`     | `TotalLatency`    | as above                                                |
| `UpstreamHealthCache`    | `UpstreamHealth`  | `Cache` - it _is_ the cache; the doc says so            |
| `InlineCCRHits`          | `InlineHits`      | `CCR` - see the doctrine gap below                      |
| `InlineCCRMisses`        | `InlineMisses`    | as above                                                |
| `SSEStreamErrors`        | `StreamErrors`    | `SSE` - every stream on this path is SSE                |

**`Proxy::Threshold`** - `Const` (`Cache`, `Token`, `InitialExpand`, `Inline`,
`CacheCap`, `ResponseCap`, `InitialRatio`, `InitialFill`) and `Struct`
(`ExpandLimit`, `BodyLimit`). Each const's doc names its `[compression]` key and
its environment variable.

**The one path change.** `ChatCompletionsPath` was a route literal living among
the compression defaults. It moved to `Proxy::Request::Route::Chat::Path` - a
new `Chat` leaf under the route it names - and its five consumers were
repointed. The const's doc names the literal and the normalisation contract.

**Local aliases** (`use ... as X`) became the word the path leaves them:
`Classify`, `Line`, `Rule`, `Parse`, `Compress` (or `Chat` where a local
`Compress` helper already existed), `Execute`, `RelayRequest`/`RelayResponse`
(`Request`/`Response` would collide with axum's `Response` and with locals named
`Request`/`Response`), `Get`, `TooLarge`, `Url`, `CopyHeaders`, `Estimate`,
`Select`, `Truncate`, `Format`, `Body`, `Create`, `Reload`, `Shape`.

**Private helpers and locals** - `IsChat`, `Suffix`, `Elapsed`, `State`,
`FillText`, `Update`, `Compute`, `Ratio`, `Backoff`, `Payload`, `Spawn`, `Lock`,
`WithCCR`, `FailingCCR`, `WithLimit`, `FailingStore`.

**Tests** - each `#[test] fn` lost the `Test` prefix its `Test/` path carries and
the case-name prefix the module carries: `TestDetectContentTypeRustCode` →
`Test::Detect::RustCode`, `TestCompressChatCompletionBelowThresholdIsNone` →
`Test::Compress::Below`, `Regression07TokensSavedIncrementsOnCompress` →
`Test::Regression::Case07` (the number is the identity; the defect is in the doc
comment above it). 78 test functions renamed.

## The three contracts

1. **serde keys - no pin was required, and none was added.** Every `Deserialize`
   and `Serialize` struct in this subtree already carries an explicit
   `#[serde(rename = ...)]` on every field (18 pins across
   `CCR::Create::Request`, `CCR::Create::Response`, `CCR::Notification::Event`,
   `Relay::Request`, `Relay::Response`). Exactly one renamed field is
   serialised: `CCR::Create::Response::Struct::Ratio`, emitted as
   `token_savings_ratio`, and its pre-existing pin is byte-identical afterwards.
   No field in this subtree is deserialised from `aphrodite.toml`, so no
   configuration key moved - `Configuration/**` is not in this partition.
2. **CLI flags - none.** No `#[arg]` in `Proxy/**`; `--help` cannot change here.
3. **ABI symbols - none.** No `pub extern "C"` in `Proxy/**`; the symbol count
   is untouched by this card.

Verified by reading the emitted-name surfaces, not the source: the pins above,
`grep -c 'arg(long' = 0`, `grep -c 'pub extern "C"' = 0`.

## Acceptance

```
grep -rnoE '\b[A-Z][a-z0-9]+([A-Z][a-z0-9]+){2,}\b' crates/aphrodite/Source/Proxy
```

Six distinct hits, all dependency/std names: `NonZeroUsize`, `AsyncReadExt`,
`AsyncWriteExt` (std/tokio), `DefaultBodyLimit` (axum), `InMemoryCcrStore`,
`SqliteCcrStore` (headroom_core). The `Env|Cfg|Impl|Num|Dir|Msg|Ctx`
abbreviation sweep is 0. No `pub trait` exists in this subtree, so the
single-implementation collapse has nothing to act on.

A stricter sweep than the card's regex - one that keeps acronym runs together,
so `InlineCCRHits` and `TestSSEResponseIsStreamed` are visible - leaves only
`AtomicU64`. That sweep is what wave 2 of the rename fixed; it is worth
repeating in the verdict, because the card's regex cannot see a compound whose
middle word is an acronym.

### Ad-hoc verification (not suite green)

No build runs in this card, so the invariants are proven by a throwaway harness
left in the OS temp directory (`hermes-verify-*-naming-b2.py`, 13 checks, exit
0). It asserts, over every `.rs` file in the subtree:

| #   | Check                                                                    | Result |
| --- | ------------------------------------------------------------------------ | ------ |
| A   | no old identifier survives anywhere in `Proxy/**`, token-exact           | pass   |
| B   | every new name still has a definition or alias site                      | pass   |
| C   | `rustfmt --edition 2024 --check` parses and agrees on every changed file | pass   |
| D1  | serde pin count is 18, unchanged                                         | pass   |
| D2  | `Ratio` still sits on `rename = "token_savings_ratio"`                   | pass   |
| D3  | no `arg(long` in the subtree                                             | pass   |
| D4  | no `pub extern "C"` in the subtree                                       | pass   |
| E1  | acceptance sweep is allowlist-only                                       | pass   |
| E2  | `Env\|Cfg\|Impl\|Num\|Dir\|Msg\|Ctx` sweep is empty                      | pass   |
| E3  | acronym-aware sweep is allowlist-only                                    | pass   |
| F1  | `Request/Route/mod.rs` declares `pub mod Chat;`                          | pass   |
| F2  | `Chat/Const.rs` defines `Path` with the literal                          | pass   |
| F3  | five consumers import `Chat::Path`                                       | pass   |

Three of those checks failed on the first run and all three were harness bugs,
not tree bugs - worth recording because two are the trap class this note already
warns about: `re.findall` on a grouped pattern returns the group rather than the
match (so `[A-Z][a-z0-9]+(...){2,}` "found" `Ext`, `Limit`, `Store`, `Usize`);
`\bconst\s+(\w+)` captures `fn` from `const fn InitialFill`; and a bare-token
scan counted `crate::Marker::Parse::...` - another partition's module - as an
identifier of mine.

**Not proven here**: that the crate compiles, and that the fifteen files listed
under "Out-of-row repointing" compile. That is the first-build card's job.

## Words that kept a qualifier, and why

- `RequestBytes`/`ResponseBytes`, `UpstreamLatency`/`TotalLatency`,
  `CacheThreshold`/`TokenThreshold`/`InlineThreshold`, `RelayCalls`/
  `RelaySuccess`/`RelayFailure`, `ConnectErrors`/`OtherErrors` - two tokens each.
  One word would collide inside the same struct (§4): `Bytes`, `Latency`,
  `Threshold`, `Relay`, `Errors` each name two or three different fields.
  The qualifier that survived is the one naming the distinction.
- `TestState`, `TestRequest`, `HeaderValue`, `FailingCCR`, `InlineCCR` - two
  tokens, allowed; each is a fixture whose single word would shadow the type it
  builds.
- `pub inline:usize` in `Threshold::Struct` stays lowercase and unrenamed: it is
  not a compound, and `Inline` beside the `Inline` const would read worse.

## Out-of-row repointing this card could not do

Twelve renamed identifiers are referenced outside `Proxy/**`. Each needs the
owning card (or the verification card) to repoint it; the new name is in
brackets.

- `AutoExpandLimit` [`ExpandLimit`] - 18 references:
  `State/Struct.rs`, `State/Default.rs`,
  `ConfigurationLoader/Configuration/Apply/Section/Runtime.rs`,
  `Debug/Summary/Fn.rs`, `Hook/AutoExpand/Applies/Fn.rs`,
  `Hook/Terminal/Policy/Struct.rs`, `Hook/ToolResult/Policy/Struct.rs`,
  `Hook/Test/{Live,Terminal,ToolResult}.rs`, and
  `aphrodite-hermes/Source/Hook/Call/Dispatch/PreTool/Fn.rs`.
- `MaximumRequestBodySize` [`BodyLimit`] - `State/Struct.rs`, `State/Default.rs`,
  `ConfigurationLoader/Configuration/Apply/Section/Runtime.rs`.
- `UpstreamHealthCache` [`UpstreamHealth`] - `Boot/Route/Health/Upstream.rs`
  (3 references, including a doc comment).
- `CacheCompressThreshold` [`CacheThreshold`], `TokenCompressThreshold`
  [`TokenThreshold`], `InlineCCRThreshold` [`InlineThreshold`],
  `CodeMultiplierX100` [`CodeMultiplier`] - all four in
  `Boot/Listen/Watcher/Apply/Fn.rs`.
- `InlineCCRHits` [`InlineHits`], `InlineCCRMisses` [`InlineMisses`] -
  `Retrieve/Handle/Lookup/Fn.rs`.
- `HandleCCRCreate` [`Create`], `HandleCCRReload` [`Reload`] -
  `Boot/Route/CCR.rs` (alias and both `post(...)` sites).
- `TestStateWithCCR` [`WithCCR`] - `Retrieve/Test/{Handler,Live}.rs`
  (5 references, fully pathed).

Also repaired inside this row, because another card renamed the declaration and
left the consumer: `Test/Threshold/Fn.rs` and `State/Build/Fn.rs` referenced
`Configuration::Proxy::Compression` fields by their pre-rename names
(`EngineThresholdPct`, `EngineProtectFirst`, `EngineProtectLast`,
`EngineMinimumMsgs`, `ToolThresholdToken`, `ToolThresholdCache`,
`TerminalThreshold`, `CatalogMode`, `ClassifierPoll`, `StoreMaximumEntries`,
`StoreMaximumBytes`, `InlineThreshold`, `AutoExpand`, `AutoExpandLimit`,
`MaximumRequestBodySize`, `CodeMultiplier`) - all now use the one-word names the
configuration struct actually has.

## What the doctrine does not cover

1. **The acceptance regex cannot see acronym-headed compounds.** `[A-Z][a-z0-9]+`
   does not match `CCR`, `SSE`, `JSON`, `API`, `URL`, so
   `TestSSEResponseIsStreamedWithHeaderAndNotCached` and `InlineCCRHits` pass the
   gate while being exactly the defect the doctrine forbids. Any sweep that
   relies on that regex alone will report a clean tree that is not clean. The
   doctrine should name a splitter (acronym runs stay together) rather than a
   regex.
2. **"Add the name to the declared allowlist (§7)" has no allowlist.** §7 is
   titled "What this doctrine is not" and contains no list, so a two-token name
   kept for a stated reason has nowhere to be recorded. The reasons are in the
   doc comments here; a machine-readable allowlist would be better.
3. **Fields have no path to push into.** The doctrine's worked example moves
   `PreviewMaximumChars`'s extra words into `Preview::State`, which works because
   `State` is the struct's path. For a _field of a struct that already sits at its
   own leaf_, there is no further path, so the words can only go into the doc
   comment. The doctrine should say so, otherwise every field rename looks like
   the failure mode the card warns about.
4. **Multi-item leaves.** `Threshold/Const.rs` holds eight consts in one file, so
   "the words move into the path" would require eight directories. This card
   treated that as restructuring, not renaming (the card's own "not a rewrite"
   rule), and put the words in the doc comments instead. If per-item paths are
   wanted for constants, that is a separate card.
5. **Log field names are a fourth external surface.** `CCR/Reload/Fn.rs` emits
   `tracing` fields named after the identifiers (`CacheCompressThreshold =
Thresholds.Cache`), so this rename changed those log keys to
   `CacheThreshold`/`TokenThreshold`/`InlineThreshold`/`ExpandLimit`/`BodyLimit`.
   §5 lists serde, clap and ABI but not tracing field names; either they are a
   contract to pin or they are free, and the doctrine should say which.
