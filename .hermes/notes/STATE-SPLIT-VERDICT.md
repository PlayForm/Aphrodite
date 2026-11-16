# The state split: verdict

`R1` moved the caps out of `State/Const.rs` into their owners and `R2` split `AphroditeState` into
per-domain sub-structs. Neither card was allowed a build, so this is the first compile of both. Both
close: seven gates green, four structural proofs hold, no design was re-run. Nineteen files needed
mechanical repair to get there, and every one of them was a stale path or a seed the compiler or the
linter refused - none was a decision.

---

## The ladder

Run in the card's order, each to its own exit code and never through a pipe:

- `cargo ci-check` - **0**
- `cargo test --workspace` - **0**
- `cargo ci-clippy` - **0**
- `cargo ci-fmt` - **0**
- `python3 Maintain/check_ffi_contract.py` - **0**
- `python3 crates/aphrodite-hermes/codegen/test_finalize_bindings.py` - **0**
- `diff -q plugins/aphrodite/__init__.py crates/aphrodite/templates/__init__.py` - **0**

One gate is not on the card and was still required: `cargo ci-check` is `check --workspace --release`,
which does not compile `#[cfg(test)]` bodies, so `cargo check --workspace --release --all-targets` was
run first and also exits **0**. Without it the test-target breaks below would only surface inside
`cargo test`, one round of errors at a time.

---

## Proof 1 - every field resolves through a sub-struct

`State/Struct.rs` declares nine fields and every one is a sub-struct:

- `Store`, `Marker`, `Event`, `File`, `Turn`, `ChainSplit`, `Directive`, `PollWorker`,
  `Configuration` - nine fields, nine sub-structs, zero scalars.

No `State.<scalar>` access survives in either crate, and the evidence is the compiler rather than a
grep: a stale scalar path is a `no field` error, and the release build of both crates (all targets)
resolves every field access in the tree with 0 errors.

The name-resolution probe written by `R2` agrees, independently of the compiler:

```sh
python3 .hermes/scripts/state_verify.py
echo "EXIT:$?"
```

51 fields over 9 domains, 0 fields lost. Its 17 "unknown" hits are all fields of a _different_ type
that happens to share a domain word - `Target.File.display`, `Segment.Marker.len`,
`Templates.Marker.get`, `Row.Marker.max`, `self.Store.clone` - which is why the probe is a net and the
build is the proof.

---

## Proof 2 - the caps are one-word and owned

```sh
grep -rn 'RecentMarkersCap\|ToolEventCap\|ReferencedFilesCap\|SplitEventCap\|SplitAdaptMinimumEvents\|DefaultInlineByteBudget' \
	crates/aphrodite/Source crates/aphrodite-hermes/Source plugins/aphrodite --include=*.rs --include=*.py | wc -l
```

Returns **0**. `State/Const.rs` is deleted, and each constant now lives beside the structure it bounds,
one word, same value:

- `RecentMarkersCap` 200 → `State::Marker::Const::Cap` 200 - `State/Marker/Const.rs`
- `ToolEventCap` 200 → `State::Event::Const::Cap` 200 - `State/Event/Const.rs`
- `ReferencedFilesCap` 100 → `State::File::Const::Cap` 100 - `State/File/Const.rs`
- `SplitEventCap` 16 → `State::ChainSplit::Const::Cap` 16 - `State/ChainSplit/Const.rs`
- `SplitAdaptMinimumEvents` 4 → `State::ChainSplit::Window::Const::Minimum` 4 -
  `State/ChainSplit/Window/Const.rs`
- `InlineMaximum` 500 → `State::Store::Entry::Const::Cap` 500 - `State/Store/Entry/Const.rs`
- `DefaultInlineByteBudget` 256 MiB → `State::Store::Const::Budget` 256 MiB - `State/Store/Const.rs`

Seven, not the card's six: `State/Const.rs` held seven constants, and the card's count came from the
six named in the sweep. The seventh (`InlineMaximum`) moved with them, and the store's two bounds now
sit one level apart (`Store::Entry::Const::Cap` for entries, `Store::Const::Budget` for bytes) because
they bound different things.

---

## Proof 3 - authority narrowed

Three free functions that took the whole state now take one sub-struct. Before and after, read from
the pre-split commit:

- `Session/Archive/Fn.rs:23` - `pub fn Fn(Turn:&mut Turn, hash, Summary, Size)`; was
  `pub fn Fn(State:&mut AphroditeState, hash, Summary, Size)`.
- `Session/Turn/Fn.rs:14` - `pub fn Fn(Turn:&mut Turn) -> usize`; was
  `pub fn Fn(State:&mut AphroditeState) -> usize`.
- `Session/Index/Fn.rs:13` - `pub fn Fn(Turn:&Turn) -> Vec<serde_json::Value>`; was
  `pub fn Fn(State:&AphroditeState) -> Vec<serde_json::Value>`.

Eleven `impl AphroditeState` blocks went with them, each now an `impl Struct` in the domain file that
owns the data, so the receiver - not a comment - is the capability:

- `Store::{Put, Get, Contains, Set, Evict}` - `State/Store/{Put,Get,Contains,Set,Evict}.rs`
- `Marker::Record` - `State/Marker/Record.rs`
- `Event::Record` - `State/Event/Record.rs`
- `File::Record` - `State/File/Record.rs`
- `ChainSplit::{Record, Retrieve, Adapt}` - `State/ChainSplit/{Record,Retrieve,Adapt}.rs`

`Store::Put` alone has 79 call sites, so the narrowing is not a decorative signature: the store's
mutators can no longer reach the marker ring, the turn counter or the configuration.

---

## Proof 4 - behaviour unchanged

`cargo test --workspace` - 674 tests, 0 failed, 1 ignored, across ten targets: 558 in the `aphrodite`
lib, 17 in its bin, 5 + 2 + 4 + 2 in the four integration suites, 83 in the `aphrodite-hermes` lib, 3
doc-tests. The eviction, cap, byte-accounting and index-order tests are the ones that would move if a
default or an ordering had shifted, and they are green.

Serialised fields: the only State type that reaches a wire shape is `Marker::Entry`, and its keys are
already pinned one by one with `#[serde(rename = ...)]` (`ccr_type`, `size`, `preview`, `turn`,
`center`). The type moved to `State/Marker/Entry/Struct.rs` unchanged, so no key needed re-pinning -
`R2`'s claim that no split field is a wire key holds. No other struct under `State/` derives
`Serialize`.

Defaults: the seed values are the same numbers and the same switches, with two deliberate exceptions
that stay off (`ChainSplit::Enable`, `Configuration::AutoExpand`) so a missing config can never enable
a feature. Five trivial seeds became `#[derive(Default)]` (see below) - the derive emits exactly the
`HashMap::new()` / `Vec::new()` / `0` / `None` the manual blocks wrote.

---

## The control-flow sweeps

```sh
grep -rnE '(^|[^A-Za-z0-9_])if[ (]' crates/*/Source --include=*.rs | grep -vE '^\S+:[0-9]+:\s*//' | wc -l
grep -rnE '^\s*for ' crates/*/Source --include=*.rs | grep -vE '^\S+:[0-9]+:\s*//' | wc -l
grep -rn 'filter(|_| ' crates/*/Source --include=*.rs | wc -l
```

- `if` - **24** against `CFV`'s 24
- `for` - **1** against 1 (`Retrieve/Test/Handler.rs:36`, a test fixture)
- `filter(|_| ` - **1** against 1 (`Tool/Unwrap/Matcher/Sole/mod.rs:11`, a doc comment recording the
  matcher that was removed)

No rise, so no file to name. Neither card is control-flow work and neither reintroduced a conversion.

---

## What the first build had to repair

Nineteen files, four groups, no design change.

**The Proxy state was renamed one struct too far.** Five sites were rewritten to
`State.Configuration.URL` / `State.Configuration.AutoExpand`, but the Proxy's `Struct` is a different
type with its own field names (`APIURL`, `AutoExpand`) and its own split is explicitly deferred. The
`APIURL`→`URL` rename belongs to the _configuration domain_, not to every struct holding a URL.
Reverted at `Proxy/Handle/Proxy/Route/Url/Fn.rs:20`, `Proxy/Test/Stream/Fn.rs:66,102`,
`Proxy/Test/Compress/Fn.rs:99,128,143`, `Boot/Route/Health/Upstream.rs:82`, and the field list in
`Proxy/State/Struct.rs`.

**One consumer still passed the whole state to a narrowed function.** `Session/Engine/Apply/Fn.rs:30`
called `Archive(State, ...)`; it now passes `&mut State.Turn`.

**Three test fixtures kept the pre-split call shape.** `State/Test/Marker.rs:4` imported
`Marker::Struct::Struct` where it needed the entry (`Marker::Entry::Struct::Struct`);
`Session/Test/Archive.rs` passed `&mut s` to `Archive` (7 sites) and `&s` to `Index`;
`Session/Test/Turn.rs:8-9` passed `&mut s` to `Turn`.

**Clippy refused five hand-written seeds.** `derivable_impls` fires when a `Default` impl's body is
exactly the derive: `AphroditeState`, `Directive`, `Event`, `Marker` and `Turn` were all that shape, so
each became `#[derive(Default)]` on the struct and its `Default.rs` leaf was deleted (the seed sentence
moved into the struct doc and the facade bullet). The five seeds with real content - `Store`, `File`,
`ChainSplit`, `PollWorker`, `Configuration` - keep their own files, because their defaults are values,
not `Default::default()`.

---

## The qualifiers that survived

Twenty-one of the 25 `Configuration` fields keep a subsystem qualifier, because `State::Configuration`
says _that_ these are configuration, not _which_ subsystem each knob belongs to:

- thresholds and engine bounds - `ToolThreshold`, `TerminalThreshold`, `EngineThresholdPct`,
  `EngineMinimumMsgs`, `EngineProtectFirst`, `EngineProtectLast`, `ContextEngineEnabled`
- flow and injection - `FlowBudgetChars`, `SessionInject`, `RetrieveGuidance`, `CatalogIntentHints`,
  `LiveContainer`, `RecursiveDepth`, `MaximumRequestBodySize`
- behaviour switches - `AutoExpand`, `AutoExpandLimit`, `AutoReload`, `DebugLevel`, `CatalogMode`,
  `ExpandGuidance`, `DevMode`

Four are already one word and dropped nothing: `URL`, `Model`, `Error`, `Templates`. Stripping the
qualifier from the rest would have collapsed `ToolThreshold`, `TerminalThreshold` and
`EngineThresholdPct` onto one word inside a single struct, which the naming doctrine's collision rule
forbids.

Three of the survivors - `CatalogMode`, `ExpandGuidance`, `DevMode` - are write-only: loaded and
seeded, never read. That is not a regression from the split; the pre-split tree had the same three
declarations and no reader, and they are marked RESERVED in the struct doc for the context-engine
work.

---

## Verdict

- **`R1` - CLOSED.** Seven constants, seven owners, one word each, values unchanged, the old names gone
  from every crate and from the plugin, and `State/Const.rs` deleted.
- **`R2` - CLOSED.** Nine sub-structs over all 51 fields, no scalar field left on `AphroditeState`, no
  stale scalar path anywhere in either crate, eleven `impl` blocks and three free functions narrowed to
  one domain each, and the 674-test suite green. The Proxy over-reach was a mechanical misapplication
  of a correct rename and is reverted, not a defect in the split.
