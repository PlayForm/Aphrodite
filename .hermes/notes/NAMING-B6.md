# Naming B6 - runtime state

The one-word pass over `State/`, `Session/`, `Prefetch/`, `PollWorker/` and `Debug/`: 196
identifiers renamed, 207 files touched, and the acceptance grep in the partition fell from 659
hits to 258 - every survivor is named below with the reason it survives.

## The rule as applied

| Surface  | Before                                               | After                          | Where the words went                      |
| -------- | ---------------------------------------------------- | ------------------------------ | ----------------------------------------- |
| type     | `State::Marker::Struct::MarkerEntry`                 | `Struct` (consumer: `Entry`)   | the path already said `Marker`            |
| type     | `PollWorker::Task::Struct::BgTask`                   | `Struct` (consumer: `Record`)  | the path already said `PollWorker::Task`  |
| type     | `State::Identity::Command::Struct::CommandID`        | `Struct` (consumer: `Command`) | the path already said `Command`           |
| function | `PollWorker::Insert::Fn::InsertBgTask`               | `Fn` (consumer: `Insert`)      | the directory already said `Insert`       |
| function | `Session::Start::Fn::OnSessionStart`                 | `Fn` (consumer: `Start`)       | the directory already said `Start`        |
| method   | `AphroditeState::InlineStorePut`                     | `Put`                          | `State::Store::Put` names it              |
| constant | `PollWorker::Output::Const::MaximumAccumulatedBytes` | `Maximum`                      | `PollWorker::Output` names it             |
| variant  | `Engine::Gate::Enum::MiddleTooSmall`                 | `Small`                        | the sibling `OutOfRange` carries the rest |

A consumer keeps the distinction, never the domain: `use ...::Marker::Struct::Struct as Entry`
because the file is already inside the marker catalogue.

## The three contracts - none moved

- **serde keys.** No `Deserialize` or `Serialize` struct had a field renamed; only type names
  moved. `State/Marker/Struct.rs` and `State/Template/Struct.rs` were compared key-set to
  key-set against the previous revision - identical. The one field rename
  (`PollEvent::ExecutionID` → `Execution`, `BgTask::ExecutionID` → `Execution`) touches
  `#[derive(Debug, Clone)]` types only, so no wire key exists for it.
- **CLI flags.** The partition carries no clap surface at all.
- **ABI symbols.** The partition carries no `extern "C"` export; the 24 core symbols live in
  `ABI/` and were not edited.

**The reviewer's list of pins to check is therefore empty** - and that is a claim about the
emitted names, so it is worth re-deriving rather than trusting: `grep` the partition for
`serde(rename`, `arg(long` and `extern "C"` before and after.

## What kept a qualifier, and why

| Name                                                       | Reason                                                                                                             |
| ---------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `RecordMarker`, `RecordEvent`, `RecordFile`, `RecordSplit` | four record operations on one receiver - one scope, so the collision rule (§4) keeps the object as the distinction |
| `OutOfRange`                                               | a bound-pair failure that has no single word; the pair with `Small` is the distinction                             |
| `ObservedAtUnix`, `ArrivedAtUnix`, `LiveArrivedAtUnix`     | a timestamp is a quantity plus its unit; dropping `Unix` would leave a number whose scale the reader has to guess  |
| `AphroditeState` fields                                    | see the doctrine gap below                                                                                         |
| `State::Const` caps                                        | see the doctrine gap below                                                                                         |

## The doctrine does not cover two things here

**A god-struct has no path to move its words into.** `State::Struct::AphroditeState` carries 45
fields spanning six domains: `ChainSplitMinimumSegments`, `EngineThresholdPct`, `SplitSegmentMap`,
`LastEmittedMarkerCount`, `PollWorkerEnabled`, `MaximumRequestBodySize`. The path is `State::Struct`,
which says none of those words, so renaming any field to one word (`Minimum`, `Threshold`, `Map`)
would delete the distinction rather than relocate it - the failure mode the card names. The
doctrinal fix is a structural one: split the fields into per-domain sub-structs
(`State::ChainSplit::Struct`, `State::Engine::Struct`, `State::Catalog::Struct`) so each path
carries the words. That is a rewrite, not a rename, and it is the single largest remaining block
of compound names in the partition (≈190 of the 258 surviving hits).

**A shared constants file has no domain in its path.** `State/Const.rs` holds the caps for six
different structures because they are one decision about a session's memory profile - the module
doc says so deliberately. Each name repeats its owner (`SplitEventCap`, `ToolEventCap`,
`RecentMarkersCap`, `ReferencedFilesCap`, `SplitAdaptMinimumEvents`, `DefaultInlineByteBudget`),
and the fix is to move each constant into its owner's directory as `Const::Cap` / `Const::Minimum`.
That is a file move plus a policy split, so it belongs to a card that can take the structural
decision.

## Work this card needs from other partitions

The renames land in this partition; 54 files outside it still spell the old names. Each is a
mechanical substitution - the leaf path, plus the consumer's own alias:

- `MarkerEntry` → `...::Marker::Struct::Struct as Entry` (10 files, `Catalog/`, `ABI/Compress`,
  `Hook/`, `ChainSplit/Prepared/Apply`, `Flow/Test/Support`, `aphrodite-hermes/Tool/Compress`)
- `TemplatesConfiguration` → `...::Template::Struct::Struct as Templates` (10 files, `Template/`,
  `ConfigurationLoader/`, `Configuration/Proxy/Multi`, `Catalog/Test`, `Flow/Test/`)
- `InlineStorePut` → `.Put(`, `InlineStoreGet` → `.Get(` (12 + 3 files, `ABI/`, `Hook/`, `Resolve/`,
  `ChainSplit/Prepared/Apply`, `aphrodite-hermes/Tool/`)
- `OnSessionStart` → `Session::Start::Fn::Fn` (4 files, `ABI/`, `Hook/Session`)
- `PrefetchStatus` → `Prefetch::Status::Fn::Fn` (4 files, `aphrodite-hermes/Schema`, `Tool/Registry`)
- `EngineMaybeArchive` → `Session::Engine::Fn::Fn` (3 files, `Flow/Engine/Offload/`)
- `ToolEvent` → `Call`, `RecordToolEvent` → `RecordEvent` (2 files, `Flow/Test/Window`,
  `Hook/Observation/Apply`)
- `BgStatus` → `Status` (2 files, `Flow/TurnContext/Collect`, `Hook/Test/Session`)
- `NoteSplitRetrieval` → `NoteRetrieval` (2 files, `ABI/Retrieve`, `Resolve/One`)
- `NextTurn` → `Turn`, `ArchiveTurn` → `Archive`, `ExpireStaleTasks` → `Expire`,
  `CheckBgTasks` → `Check`, `InsertBgTask` → `Insert`, `CatalogSummary` → `Catalog`,
  `RenderBgTaskStatus` → `Render`, `ReadPaths` → `Paths`, `InsertOutcomes` → `Insert`,
  `RecordChainSplit` → `RecordSplit`, `AdaptChainSplitThreshold` → `Adapt`,
  `StaleTurnAge` → `Age` (one file each)
- `BgTask` → `Record`, `ShouldBackgroundPre` → `Before` (`aphrodite-hermes/Hook/Call/Dispatch/PreTool`)
- `ActiveDirective` → `Nudge`, `ReadOutcome` → `Outcome`, `ContentHash` → `Digest` (one file each)

`PollWorker/Check/Fn.rs` also had to follow a rename another partition had already made
(`Flow::Nudge::Push::Fn::PushNudge` → `...::Fn::Fn as PushNudge`) - the leaf is `Fn` now, so the
import names it.

## How the result was checked without a build

The card forbids the build, so four static checks stand in, each with its own probe:

| Claim                                                       | Probe                                                            | Result                                                                             |
| ----------------------------------------------------------- | ---------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| every file still parses                                     | `rustfmt --edition 2024 --emit stdout <file>` over all 342 files | 0 failures                                                                         |
| every `use crate::...` path names an item its leaf declares | `.hermes/tmp/resolve_b6.py`                                      | 411 paths, 0 problems                                                              |
| no renamed name survives, no alias is imported twice        | `.hermes/tmp/verify_b6.py`                                       | 9 survivors, all the `Apply::Applied` **module** segment; 0 duplicate aliases      |
| the module tree resolves                                    | every `pub mod X;` has a matching directory or file              | clean                                                                              |
| the partition is format-clean                               | `rustfmt --edition 2024 --check <file>` over all 342 files       | 0 failures (20 files were reformatted after the longer `Struct as X` import lines) |

The duplicate-alias check is the one that caught real breakage: two renames collapsed to `Command`
in one scope, and one file imported `Struct as Status` beside a module named `Status`.
