# CF - control flow and construction

Every `if` statement in `crates/*/Source` became a decision expressed as a value,
and every `for` loop became a chain over the collection it walked. What follows
is the accounting the card asks for: the four counts per partition, the types
that replaced the branches, the residue that survives with its reason, and the
sites where a loop is the honest answer.

## The counts

Measured with the card's own probes. "Before" is the dispatch measurement plus
the lines this pass removed (`git diff -U0` over the partition); "after" is the
tree as it now stands.

| Partition   | `if` before | `if` after | `for` before | `for` after | `then` before | `then` after | `filter` before | `filter` after |
| :---------- | ----------: | ---------: | -----------: | ----------: | ------------: | -----------: | --------------: | -------------: |
| boundary    |          16 |          1 |            0 |           0 |             0 |            0 |               0 |              0 |
| proxy       |          15 |          2 |            2 |           0 |             2 |            0 |               1 |              0 |
| hook        |          12 |          4 |            1 |           0 |             6 |            0 |               0 |              0 |
| analysis    |          20 |          3 |            4 |           0 |             0 |            0 |               3 |              0 |
| boot        |          48 |          6 |            2 |           0 |             4 |            0 |               0 |              0 |
| state       |          14 |          3 |            0 |           0 |             3 |            0 |               0 |              0 |
| content     |           2 |          0 |            3 |           1 |             7 |            0 |               0 |              0 |
| hermes tool |           9 |          4 |            0 |           0 |             0 |            0 |               0 |              0 |
| hermes core |           5 |          1 |            0 |           0 |             2 |            0 |               0 |              0 |
| **total**   |     **141** |     **24** |       **12** |       **1** |        **24** |        **0** |           **4** |          **0** |

All four counts fell. The 24 remaining `if` hits are 16 match guards and 8
prose strings; the single remaining `for` is the declared `.await` residue; the
single remaining `filter(|_|` hit is a doc comment describing the regression that
idiom caused. None of the 24 is a branch.

## The vocabulary this pass established

Each of these replaces a shape, and each is already in the tree several times
over, so the next pass has a name for the thing rather than a pattern to
re-derive.

- **Effect gate.** The condition becomes an `Option` carrying what the effect
  needs; the terminal consumer is the only place the effect happens.
  `Out.is_none().then_some([Misses, Warnings]).into_iter().flatten().for_each(Bump)`
- **The collection decides.** A conditional push becomes a source the collection
  is fed from: `Parts.extend((!Text.is_empty()).then_some(Part { .. }))`.
- **Two-branch value.** `if cond { a } else { b }` becomes
  `cond.then_some(a).unwrap_or(b)` - the closure never holds the effect.
- **Named predicate.** `if p(x) { .. }` inside a walk becomes
  `.filter(Select::Fn::Fn)`, so the predicate has a home and a name.
- **Search, not loop.** A loop with an early exit becomes `find` / `find_map` /
  `try_fold` with `ControlFlow`; a "keep dropping until it fits" loop becomes a
  search over the prefixes it was walking anyway (`Flow::TurnContext::Budget`).
- **Step list.** A `for` over steps becomes
  `Steps.iter().try_fold(Initial, |acc, Step| Apply::Fn::Fn(&mut acc, Step).map(|()| acc))`.
- **Accumulator.** A loop carrying two facts becomes a struct with a named
  `Step` and a `Finish` (`Resolve::Recursive::Walk::Build::Writer`).

## Construction: what was introduced

Nothing here is a builder: no struct in the tree had "optional parts applied in
several places" left to fold, and the card forbids builder-ifying a struct whose
fields are all required. What the pass introduced instead are **tables**,
**factories with private fields**, and **outcome values**.

| Type                                                           | The field or condition that made it necessary                                                                                                                                                      |
| :------------------------------------------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Boot::Route::Metrics::Table::Const::ORDER` + `Piece`, `Entry` | 26 stats members, each rendered by its own `if let Some(..)`. `Entry` holds `Name`, `Path`, `Kind`, `Label`; the table is the one place "which metrics exist" is decided.                          |
| `Boot::Route::Metrics::Kind::Enum`                             | Four number shapes - `Shown` (printed as held, `null` included), `Count`, `Ratio`, `Seconds`. The variant is also the absence policy, which the original spelled as two different idioms.          |
| `Boot::Route::Metrics::Label::Enum`                            | Three label shapes - none, a literal wire value (`code="4xx"`), a value read from the document (`mode`).                                                                                           |
| `Boot::Route::Metrics::Node::{Read, Find}`                     | The lookup itself differs: `Read` walks with `Index` (always answers) for the always-present counters, `Find` walks with `get` (may answer `None`) for the optional ones. Two policies, two names. |
| `Boot::Route::Metrics::Histogram::Bucket`                      | The histogram fold carries a running total **and** the lines written so far - two values that must travel together.                                                                                |
| `Setup::MacOS::Outcome::Enum::{Done, Warned}`                  | Each Gatekeeper tool's success was a bool and its failure carried the line to print. The warning is now the value the step returns.                                                                |
| `Setup::MacOS::{Copy, Fallback, Dylib}`                        | The three tool steps, each answering an `Outcome`; `Fn` collects them and hands the warnings to one sink.                                                                                          |
| `Setup::Execute::Apply::Fn`                                    | One plan step against the `Report` it accumulates; the failure takes the completed prefix out of the report with `map_err`.                                                                        |
| `Resolve::Recursive::Walk::Build::Writer::{Struct, Step}`      | The second walk carried `Out` and `Cursor` across iterations - two values that must travel together, so they travel as one.                                                                        |
| `Proxy::Cache::Canonical::Comma::Fn`                           | The separator is a function of the member's position, not a branch inside the walk.                                                                                                                |
| `Proxy::State::Method::Record::Cap::Fn`                        | The ring bound as one operation: `len().saturating_sub(Limit)` then `drain`, instead of "pop one when over".                                                                                       |
| `Debug::Root::Store::Fn`                                       | "Store only when the value changed" - the comparison is the decision, so it belongs with the store.                                                                                                |
| `Boot::Listen::Watcher::Select::Fn`                            | The watch's event predicate, named: kind is `Modify`/`Create` and the path **is** the resolved target.                                                                                             |
| `Session::Watcher::Select::Fn`                                 | The same predicate for the hermes-side watcher, which matches a filename rather than a resolved path.                                                                                              |
| `ConfigurationLoader::..::Section::Probe::Fn`                  | One candidate directives directory: it logs the probe, records the unusable candidate, and adopts the first existing one.                                                                          |
| `ChainSplit::Marked::Output::Line::Enum`                       | A marked-output line is a boundary or text; the split is driven by that classification instead of a boolean test with an empty arm.                                                                |
| `Boot::Route::Health::Peer::Enum`                              | The upstream probe's peer: loopback or not, as a value the route matches on.                                                                                                                       |

The only renames with call-site cost: `Setup::MacOS::Fn` became
`Setup::MacOS::Fn::Fn` (two consumers), and `Boot::Route::Metrics.rs` became a
directory (one consumer, unchanged path).

## The residue

One line each. Everything below is either not control flow or is a language
limit the style note permits by name.

### Prose that contains the word "if" (8)

- `hermes/Directive/Schema` and `hermes/Schema/Retrieve` - the tool-schema
  descriptions the model reads, which happen to contain "if".
- `hermes/Bin/SetupHelper` - a `println!` telling the operator what to run next.
- `StructExtract/Test/Go` - an assertion message.
- `Flow/Prompt/Const` (2) and `Flow/Test/Prompt` (2) - the prompt text and its
  expected copy.

### Match guards inside a structural match (16)

- `Proxy/State/Build` - `ProxyMode::Token if !CLI.NoCCRMarker` refines the
  `Token` arm; `Token` with markers disabled legitimately falls through.
- `Proxy/Analysis/Classify/Envelope` - `Some(_) if Content.contains(..)` refines
  the parsed-JSON arm.
- `Marker/Format/Vars` - the name-character class of the scan loop, with
  `_ => break` as the other arm.
- `Catalog/Size/Classify` (2) - the threshold ladder on `Bytes`.
- `Setup/Plan/Ownership`, `Setup/Dylib/Checksum` (2), `Setup/Artifact/Stage`,
  `Setup/Verify/Hermes` - `Err(e) if e.kind() == NotFound` and
  `Ok(o) if o.status.success()`, each refining one arm of a `Result` match.
- `hermes/Directive/Materialize/Write` (2) - byte-identical and
  file-not-found, the two arms that decide a file's outcome.
- `PollWorker/Test/Update/{Attribution,Poll}` (3) and
  `ConfigurationLoader/Test/Apply/Template` - `matches!(.. if ..)` in test
  assertions, which is the guard form of `matches!`.

### A `for` whose body awaits (1)

- `Retrieve/Test/Handler` - three `retrieve` requests through the axum handler
  in one runtime block. An iterator adapter cannot await, and this crate's
  `futures` dependency is optional, so the loop stays and says so in a comment.

## Where a loop had to stay

Only the site above. Every other loop in the tree had a collection underneath
it: `Setup/Execute` folds a step list, `Resolve/Walk/Build` folds occurrence
pairs, `Proxy/Cache/Canonical` walks keys and items, `Hook/Live/Object/Install`
counts the chunks a policy yields, `ConfigurationLoader/..::Directive` searches
candidate directories, `Retrieve/Live/Lookup` stops at the first missing chunk,
and the two watchers gate an event through a named predicate. `Session/Watcher/Run`
keeps a `while let` over a blocking wake - that is a loop over a channel, not
over a collection, and its exit condition is now the `Option` the step returns.

## Behaviour

Unchanged, with one named exception and one deletion:

- **Deletion, not a change.** `Marker/Parse/Valid` had a `h.len() < 8` pre-gate
  that no input could reach: an `i:`/`I:` prefix leaves `len - 2` characters, so
  a hash under 8 bytes already failed the 6-character floor the next test
  applies. The gate is gone; the reachable answers are identical.
- `Flow/TurnContext/Budget::DropToBudget` is now a search over the prefixes the
  drop loop walked. The measurement only shrinks as the prefix does, so the first
  fitting prefix is the one the loop stopped at, and "no prefix fits" is the
  overflow outcome rather than a second test on the bytes.
- `Session/Watcher/Run` composes the wake and the keep-watching policy into one
  `Step`, so the loop obeys one condition instead of two. Same events, same
  applies, same exits.
- Every other rewrite keeps the same branch for the same input, the same order,
  and the same laziness (fallbacks stayed in `or_else`/`unwrap_or_else`).

## For the first build

Nothing in this pass was compiled. The shapes that carry inference or borrow
risk, and are worth reading first if `ci-check` objects:

- `Proxy/State/Method/Record` - `Cap::Fn::Fn(&mut Errors, 100)` where `Errors`
  is a `VecDeque` behind a `MutexGuard`; `drain(..Excess)` must see the guard's
  deref.
- `Proxy/Upstream/Body` - `anyhow::ensure!` with a trailing format argument.
- `Boot/Listen/Watcher/Source` and `Session/Watcher/Source` -
  `filter(Select::Fn::Fn)` and `.map(|_| Tx.try_send(..)).into_iter().for_each(drop)`
  inside a `'static` notify closure.
- `Resolve/Recursive/Walk/Build` - `Replace` takes `&Occurrence<'_>` and the
  `try_fold` closure reborrows `&mut Traversal`.
- `Boot/Route/Metrics` - const promotion of `&["requests", "total"]` inside
  `Table::Const::ORDER`, and `Node::Read`'s `fold` returning `&'a Value`.
- `Setup/MacOS/Copy` - `map_or_else(|| Fallback::Fn::Fn(..), |Done| Ok(Done))`,
  where the two closures return the same `anyhow::Result`.
- `Flow/TurnContext/Budget` - `Sections.truncate(Fitting.unwrap_or(..))` after a
  `find` that borrowed `Sections`.
