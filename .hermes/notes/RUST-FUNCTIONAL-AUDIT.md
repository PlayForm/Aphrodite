# Rust functional-style audit

This is a read-only inventory of the two Rust crates at HEAD `a98bd3b7`: 983 `.rs` files / 34,400
lines in `crates/aphrodite/Source` (823 production, 160 test) and 250 files / 7,062 lines in
`crates/aphrodite-hermes/Source` (200 production, 50 test). `vendor/**` and `Maintain/dynamic_hooks`
are excluded; test code is counted but reported separately, so a "prod" number below never includes a
`Test/` directory or a `Test*.rs` file. Every count is produced by the command printed above it, run
against the tree exported from the commit so that uncommitted working-tree edits cannot change a
number mid-audit:

```sh
mkdir -p .hermes/tmp/audit/head
git archive HEAD crates/aphrodite/Source crates/aphrodite-hermes/Source | tar -x -C .hermes/tmp/audit/head
```

Every probe runs through `.hermes/tmp/audit/measure.py`, a scratch script that blanks comments and
string literals while preserving offsets, then matches braces to find real `match` blocks, arms and
function bodies. Grep cannot see arm structure, and it overcounts every probe that touches code,
because a `//` comment or a `"stream":true` literal reads as code to it. So the harness command is the
reproducible one for every measurement, and where a plain grep reproduces a harness number exactly,
both commands are printed. The instrument proves itself, and the note with it, in one command:

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py selftest
```

That selftest checks blanking against 1,233 files, brace matching, the arm splitter against a fixture
with one known no-op arm, probe/grep parity for the three probes a grep can reproduce, that probe 9
resolves D1 and nothing else, and that every headline count below still matches a fresh run. The
file/line figures quoted elsewhere for these trees do not reproduce at HEAD; the numbers below are the
measured ones.

The question each count answers is how much of a given shape exists and where, not whether the shape
is wrong. The rule the rewrite enforces is: turn control flow into values and lazy composition
(`Option` for absence, `Result` for failure, domain enums for decisions, iterators for sequences,
`ControlFlow` for non-error termination), interpret those values with `match`, and preserve evaluation
order, ownership, lock lifetimes and enclosing-function exits.

## Counts by module

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py summary
```

Module is the first path component under `Source/`; `(root)` is `Library.rs` and `Main.rs`. "Findings"
is the total hit count across all thirteen measurements for that module.

### `crates/aphrodite` - 983 files, 34,400 lines

| Module              | Files | Lines | Test files | Test lines | Findings (prod) | Findings (test) |
| ------------------- | ----: | ----: | ---------: | ---------: | --------------: | --------------: |
| Proxy               |   134 |  4452 |         38 |       1774 |             150 |               8 |
| Preview             |    86 |  2778 |         13 |       1013 |              59 |              14 |
| Boot                |    30 |  1429 |          0 |          0 |              54 |               0 |
| Hook                |    60 |  1363 |          5 |        608 |              34 |               0 |
| ABI                 |    32 |  1311 |          1 |        331 |              20 |               0 |
| Setup               |    22 |  1163 |          3 |        125 |              23 |               0 |
| ConfigurationLoader |    34 |   963 |         11 |        704 |              17 |               3 |
| Flow                |    47 |   944 |         11 |        559 |              26 |               0 |
| StructExtract       |    14 |   870 |          8 |        223 |              61 |               2 |
| State               |    27 |   854 |          5 |        307 |              13 |               4 |
| Configuration       |    33 |   817 |         11 |        418 |              18 |               0 |
| Debug               |    51 |   766 |          1 |         46 |              12 |               0 |
| PollWorker          |    23 |   672 |         10 |        628 |              18 |               1 |
| Template            |    40 |   661 |          1 |        140 |              10 |               0 |
| Retrieve            |    18 |   572 |          5 |        435 |              12 |               1 |
| Stage2              |    21 |   536 |          3 |        122 |              23 |               0 |
| Prefetch            |    21 |   530 |          5 |        313 |              25 |               0 |
| Marker              |    13 |   527 |          4 |        437 |              12 |               0 |
| Resolve             |    12 |   500 |          7 |        469 |              14 |               1 |
| Session             |    13 |   454 |          6 |        362 |              18 |               1 |
| Scripting           |    19 |   402 |          1 |        113 |               8 |               0 |
| Directive           |     6 |   388 |          4 |        418 |              10 |               0 |
| Home                |    24 |   323 |          1 |        106 |               6 |               0 |
| ChainSplit          |    13 |   311 |          1 |         96 |               8 |               0 |
| Guard               |    11 |   229 |          4 |        162 |               5 |               0 |
| Catalog             |     7 |   211 |          1 |        162 |               9 |               1 |
| Logging             |    10 |   184 |          0 |          0 |               2 |               0 |
| (root)              |     2 |   119 |          0 |          0 |               0 |               0 |

### `crates/aphrodite-hermes` - 250 files, 7,062 lines

| Module        | Files | Lines | Test files | Test lines | Findings (prod) | Findings (test) |
| ------------- | ----: | ----: | ---------: | ---------: | --------------: | --------------: |
| Tool          |    44 |  1144 |         11 |        670 |              20 |               2 |
| Hook          |    31 |   865 |          9 |        544 |              18 |               1 |
| Debug         |    41 |   862 |          1 |         77 |              21 |               0 |
| Schema        |    35 |   652 |          4 |        196 |               0 |               3 |
| Session       |    13 |   375 |          3 |        156 |               8 |               0 |
| Directive     |     9 |   279 |         10 |        294 |               4 |               1 |
| ABI           |    11 |   228 |          6 |        139 |               1 |               0 |
| Configuration |     7 |   166 |          2 |         51 |               1 |               0 |
| Health        |     5 |    90 |          2 |         38 |               0 |               0 |
| (root)        |     1 |    75 |          0 |          0 |               0 |               0 |
| Guard         |     2 |    69 |          2 |         20 |               0 |               0 |
| Bin           |     1 |    32 |          0 |          0 |               0 |               0 |

## Findings by pattern

### 1. Dummy scrutinees

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p1 --examples 10
# same number, grep-verified:
grep -rnE 'match (\(\)|true|false) *\{' .hermes/tmp/audit/head/crates --include='*.rs' | wc -l
```

**20 total - 18 production, 2 test. All 20 are `match ()`; there is no `match true` or `match false`
scrutinee in either crate.** The shape is a guard chain with no value to interpret: the scrutinee
carries nothing and every arm is `_ if <condition>`.

| Location                              | What is there                                         |
| ------------------------------------- | ----------------------------------------------------- |
| `ABI/Guard.rs:95`                     | `match ()` with three `_ if` bails and `_ => Ok(())`  |
| `Catalog/Size/Fn.rs:8`                | `match ()` guard chain                                |
| `Flow/Section/Label/Fn.rs:15`         | `match ()` guard chain                                |
| `Marker/Format/Entry.rs:50`           | `Safe = match ()` - unit match in expression position |
| `Preview/Builder/Test.rs:70`          | `match ()` (test)                                     |
| `Preview/Builder/Test.rs:99`          | `match ()` (test)                                     |
| `Preview/Detector/XML.rs:17`          | `match ()` guard chain                                |
| `Preview/Language/Fn.rs:21`           | `match ()` guard chain                                |
| `Proxy/Compress/Chat/Fn.rs:61`        | `match ()` guard chain                                |
| `Proxy/Handle/Proxy/Respond/Fn.rs:48` | `match ()` selecting a 5xx/4xx counter                |

### 2. Boolean match with a no-op arm

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p2 --examples 10
```

**430 total - 407 production (379 `aphrodite`, 28 `aphrodite-hermes`), 23 test.** Split of the
production hits: 243 have a boolean scrutinee (a comparison, `is_*`, `matches!`, `!`), 164 interpret
an `Option`/`Result` and throw one variant away. This is the dominant shape in the tree, and it is
the residue of an earlier statement-level conversion: `if cond { work }` became
`match cond { true => { work }, false => {} }`, which is the same control flow with a value
introduced only to be discarded. For scale:

```sh
grep -rnE '^\s*(true|false) => ' .hermes/tmp/audit/head/crates --include='*.rs' | wc -l # 956
```

so a no-op arm is present in just under half of all boolean matches.

| Location                     | What is there                                                              |
| ---------------------------- | -------------------------------------------------------------------------- |
| `ABI/Compress.rs:25`         | `match Content.is_empty() { true => return ..., false => () }`             |
| `ABI/Compress.rs:30`         | `match Content.len() > MaximumContent { true => return ..., false => () }` |
| `ABI/Dispatch.rs:42`         | same length-guard shape                                                    |
| `ABI/FreeString.rs:17`       | `match !s.is_null() { true => ..., false => {} }`                          |
| `ABI/Guard.rs:75`            | `match Ptr.is_null() { true => ..., false => {} }`                         |
| `ABI/Handle/Table.rs:34`     | `match g.is_none() { ... }`                                                |
| `ABI/Initialise.rs:40`       | `match !path.is_empty() { ... }`                                           |
| `ABI/Classify.rs:25`         | `match CheckContent(&c) { Ok(()) => {}, Err(...) => ... }` (OTHER)         |
| `ABI/ConfigurationSet.rs:30` | 5-arm string dispatch with one `_ => {}` (OTHER)                           |
| `ABI/Reload.rs:69`           | `match ContextEngineEnabled { ... None => ... }` (OTHER)                   |

### 3. Artificial-error termination

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p3 --examples 10
```

**40 total - 39 production (34 `aphrodite`, 5 `aphrodite-hermes`), 1 test.** Production split:
`Err(())` appears 18 times, `Ok::<(), ()>` once, and 18 `try_for_each` closures carry a
`Result<(), ()>`/`Err(())` payload used only as a `break`. The clearest instance is the language
scanners, where the comment states the intent: "`Err(())` is the spent-budget sentinel: it stops the
scan exactly where the `break` did, and the result is discarded because the budget, not the walk, is
the outcome."

| Location                       | What is there                                                        |
| ------------------------------ | -------------------------------------------------------------------- |
| `StructExtract/Rust.rs:32`     | `Content.lines().try_for_each(\|Line\| -> Result<(), ()>` - scan cap |
| `StructExtract/Rust.rs:35`     | `return Err(())` ending a successful traversal                       |
| `StructExtract/Go.rs:31`       | same scanner shape                                                   |
| `StructExtract/JS.rs:28`       | same scanner shape                                                   |
| `StructExtract/Python.rs:30`   | same scanner shape                                                   |
| `PollWorker/Check/Fn.rs:21`    | `(0..TaskCount).rev().try_for_each` - `Err(())` as the 2-nudge break |
| `Proxy/Metadata/Git/Fn.rs:4`   | `lines().try_for_each` + `Err(())`                                   |
| `Proxy/Metadata/JSON/Fn.rs:10` | `lines().try_for_each` + `Err(())`                                   |
| `Debug/Chain/Fn.rs:29`         | `(0..16).try_for_each` - depth cap as an error                       |
| `Session/Watcher/Fn.rs:70`     | `rx.iter().try_for_each` + `false => Err(())`                        |

### 4. Guarded match restating one `if`

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p4 --examples 10
```

**28 total - all production (19 `aphrodite`, 9 `aphrodite-hermes`), 0 test.** Each is exactly one
`if`-guard arm plus a catch-all: the guard is an `if` wearing match syntax.

| Location                            | What is there                                   |
| ----------------------------------- | ----------------------------------------------- |
| `Flow/Engine/Offload/Fn.rs:46`      | `Some(Text) if !Text.is_empty()` + catch-all    |
| `Marker/Format/Vars.rs:40`          | `Some(c) if c.is_ascii_alphanumeric() \|\| ...` |
| `Prefetch/Read/Refuse/Fn.rs:33`     | `Some(name) if name == ...`                     |
| `Prefetch/Read/Refuse/Fn.rs:40`     | `Some(Home) if Canon.starts_with(...)`          |
| `Preview/Builder/Build.rs:36`       | `Some(Message) if !Message.is_empty()`          |
| `Preview/Line/Grep/SplitPath.rs:12` | `Some(p) if !p.trim().is_empty() && ...`        |
| `Preview/Line/Grep/SplitPath.rs:16` | `Some(Mid) if Mid.chars().all(...)`             |
| `Preview/Line/Lint.rs:27`           | `Some(p) if !p.is_empty() && ...`               |
| `Preview/Type/Fn.rs:40`             | `Some(...) if !Inp.raw.lines().any(...)`        |
| `Proxy/CCR/Create/Handle/Fn.rs:129` | `Ok(r) if r.status().is_success()`              |

### 5. Eager alternative tuples

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p5 --examples 10
```

**17 total - 16 production (2 `aphrodite`, 14 `aphrodite-hermes`), 1 test.** Every one is `match (`
over a comma tuple, i.e. alternatives computed before the match. `aphrodite-hermes` carries 14 of the
16, all in the hook dispatch path where a `(flag, payload)` pair decides the arm.

| Location                                     | What is there                                                 |
| -------------------------------------------- | ------------------------------------------------------------- |
| `Configuration/Proxy/Multi/Resolve/Fn.rs:47` | `match (cfg.Mode.as_deref(), cfg.name.as_deref())`            |
| `Preview/Text/FirstMeaningful.rs:24`         | `match (It.next(), It.next())`                                |
| `Configuration/Path/Fn.rs:35`                | `match (` tuple                                               |
| `Directive/Resolve/Fn.rs:50`                 | `match (explicit_home, directive_directory, home_override)`   |
| `Hook/Call/Dispatch/PreLLM/Fn.rs:36`         | `match (` tuple                                               |
| `Hook/Call/Dispatch/PreTool/Fn.rs:44`        | `let background = match (state.PollWorkerEnabled, call_tool)` |
| `Hook/Call/Dispatch/PreTool/Fn.rs:88`        | `let chain = match (state.ChainSplitEnabled, call_tool)`      |
| `Hook/Call/Dispatch/ToolResult/Fn.rs:23`     | `match (state.PollWorkerEnabled, tool)`                       |
| `Hook/Replacement/Fn.rs:63`                  | `match (chain, expanded, marker)`                             |
| `Schema/Test/Shape.rs:60`                    | `match (pschema.get("default"), ...)` (test)                  |

### 6. `if` in expression position

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p6 --examples 10
```

**2 total - 2 production (1 per crate), 0 test. Expression-position `if` is at zero in both crates;**
both hits are statement-position `if` in a chain, and the audit's `= if `/`(if `/`, if `/`=> if `
predicates match nothing. `#[cfg]`, `cfg!()`, match guards, comments and string literals are excluded
by the harness, and a match-guard line is rejected explicitly (a guard is `pattern if cond =>`, not an
`if` expression).

| Location                     | What is there                                        |
| ---------------------------- | ---------------------------------------------------- |
| `Setup/Dylib/Checksum.rs:68` | `} else if Command::new(...).output().is_ok() {`     |
| `Session/Watcher/Fn.rs:42`   | `if matches!(ev.kind, notify::EventKind::Modify(_))` |

### 7. Sentinels and stringly-typed decisions

#### 7a. Zero sentinels

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p7a --examples 10
```

**30 total - 27 production (26 `aphrodite`, 1 `aphrodite-hermes`), 3 test.** One site is documented as
an unlimited sentinel within three lines of the comparison (`Preview/Builder/Cap.rs:15` - "`0` means
no cap - not \"truncate to nothing\""); the rest are ordinary zero comparisons. Where a zero sentinel
meets an `Option`, the mapping is consistent: `0` → `None` → `usize::MAX`
(`ConfigurationLoader/.../Global/Preview.rs:20` and `Proxy/Body/Capped/Struct.rs:39`), so the sentinel is
read the same way on both sides.

| Location                                       | What is there                                                    |
| ---------------------------------------------- | ---------------------------------------------------------------- |
| `Preview/Builder/Cap.rs:17`                    | `MaximumChars == 0` = no cap (documented)                        |
| `Preview/Builder/Cap.rs:26`                    | `MaximumChars <= 3` collapse guard                               |
| `Debug/Level/Enabled/Fn.rs:10`                 | `Level() != 0` as "enabled"                                      |
| `Debug/Log/At/Fn.rs:24`                        | `match Level == 0`                                               |
| `Debug/Engine/Skip/Fn.rs:12`                   | `match State.DebugLevel == 0`                                    |
| `Debug/Metrics/Fn.rs:24`                       | `match n == 0`                                                   |
| `Prefetch/Arrival/Struct.rs:43`                | `BytesArrived == 0 \|\| BytesArrived >= Size`                    |
| `Preview/Builder/Grep.rs:38`                   | `match Hits == 0`                                                |
| `Hook/Terminal/Record/Fn.rs:15`                | `Returncode.map(\|Rc\| Rc == 0).unwrap_or(true)` - absent = okay |
| `ConfigurationLoader/.../Global/Preview.rs:20` | `Maximum == 0` → `None` (unlimited)                              |

#### 7b. `Option<usize>` mode fields

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p7b --examples 10
# same number, grep-verified:
grep -rn 'Option<usize>' .hermes/tmp/audit/head/crates --include='*.rs' | wc -l
```

**13 total - 13 production, 0 test, all in `aphrodite`.** Seven are configuration fields where `None`
means "unbounded" and a sibling `usize` field would mean "bounded by N"; none of them is paired with a
mode enum.

| Location                                                | What is there                              |
| ------------------------------------------------------- | ------------------------------------------ |
| `Configuration/Proxy/Compression/Struct.rs:49`          | `MaximumRequestBodySize:Option<usize>`     |
| `Configuration/Proxy/Compression/Struct.rs:64`          | `StoreMaximumEntries:Option<usize>`        |
| `Configuration/Proxy/Proxy/Struct.rs:53`                | `MaximumContext:Option<usize>`             |
| `Configuration/Proxy/Proxy/Struct.rs:56`                | `MaximumOutput:Option<usize>`              |
| `Proxy/State/Struct.rs:228`                             | `MaximumRequestBodySize:Option<usize>`     |
| `Proxy/Threshold/Struct.rs:35`                          | `MaximumRequestBodySize:Option<usize>`     |
| `Scripting/Payload/Struct.rs:22`                        | `Size:Option<usize>`                       |
| `ConfigurationLoader/Configuration/Resolve/Limit.rs:14` | `GetOptUsize(...) -> Option<usize>`        |
| `State/ChainSplit/Retrieve.rs:15`                       | `NoteSplitRetrieval(...) -> Option<usize>` |
| `Flow/TurnContext/Fn.rs:44`                             | `EstRequestBytes:Option<usize>`            |

#### 7c. Stringly-typed decisions

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p7c --examples 10
```

**16 total - 16 production (14 `aphrodite`, 2 `aphrodite-hermes`), 0 test.** Each is a `match` on a
`&str`/`String` with two or more string-literal arms selecting behaviour. Three of them carry more
than ten literal arms, which is a domain enum expressed as text.

| Location                                | What is there                                            |
| --------------------------------------- | -------------------------------------------------------- |
| `ABI/Dispatch.rs:81`                    | `match name.as_ref()` - 13 literal arms (C ABI dispatch) |
| `Proxy/Metadata/Fn.rs:17`               | `match Ct` - 13 literal arms (content type)              |
| `Preview/Builder/Dispatch.rs:41`        | `match Effective` - 20 literal arms (preview arm)        |
| `ABI/ConfigurationGet.rs:30`            | `match k.as_ref()` - 5 arms                              |
| `ABI/ConfigurationSet.rs:30`            | `match k.as_ref()` - 4 arms                              |
| `Directive/Action.rs:26`                | `match Action` - 6 arms                                  |
| `Proxy/State/Method/Threshold/Fn.rs:52` | `match Ct` - 6 arms                                      |
| `Template/Vars/Code/Fn.rs:13`           | `match TypeStr` - 6 arms                                 |
| `Hook/Call/Dispatch/Fn.rs:20`           | `match name` - 6 arms (`aphrodite-hermes`)               |
| `Proxy/Relay/Execute/Fn.rs:32`          | `match Tool` - 3 arms                                    |

#### 7d. `&mut Vec<String>` accumulators

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p7d --examples 10
# same number, grep-verified:
grep -rn '&mut Vec<String>' .hermes/tmp/audit/head/crates --include='*.rs' | wc -l
```

**15 total - 15 production, 0 test, all in `aphrodite`.** Thirteen are the metadata extractors, each
`Fn(Content:&str, ..., Parts:&mut Vec<String>)`: a function that appends strings to a caller-owned vector
instead of returning a typed value.

| Location                        | What is there                              |
| ------------------------------- | ------------------------------------------ |
| `Proxy/Metadata/Build/Fn.rs:3`  | `Fn(Content:&str, Parts:&mut Vec<String>)` |
| `Proxy/Metadata/Code/Fn.rs:3`   | same signature                             |
| `Proxy/Metadata/Diff/Fn.rs:3`   | same signature                             |
| `Proxy/Metadata/Error/Fn.rs:5`  | same signature                             |
| `Proxy/Metadata/Git/Fn.rs:3`    | same signature                             |
| `Proxy/Metadata/Go/Fn.rs:3`     | same signature                             |
| `Proxy/Metadata/JS/Fn.rs:3`     | same signature                             |
| `Proxy/Metadata/JSON/Fn.rs:5`   | same signature                             |
| `Proxy/Metadata/Linter/Fn.rs:3` | same signature                             |
| `Proxy/Metadata/Log/Fn.rs:3`    | same signature                             |

### 8. Ownership churn

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p8 --examples 10
```

**86 total - 77 production (68 `aphrodite`, 9 `aphrodite-hermes`), 9 test.** 77 of the 86 are a
`.clone()` inside a combinator closure (`map`, `and_then`, `for_each`, `or_else`, `filter_map`,
`try_for_each`); 8 are `.to_vec()` calls, and one function serialises the same body twice
(`Proxy/Handle/Proxy/Respond/Fn.rs:41`, `serde_json::to_vec` ×2). `.iter().cloned()` does not appear
anywhere in either crate.

| Location                                      | What is there                            |
| --------------------------------------------- | ---------------------------------------- |
| `Configuration/Proxy/Multi/Resolve/Fn.rs:32`  | clone in `.or_else()` closure            |
| `Configuration/Proxy/Multi/Resolve/Fn.rs:87`  | clone in `.or_else()` closure            |
| `Configuration/Proxy/Multi/Resolve/Fn.rs:88`  | clone in `.or_else()` closure            |
| `Configuration/Proxy/Multi/Resolve/Fn.rs:93`  | clone in `.or_else()` closure            |
| `Configuration/Proxy/Multi/Resolve/Fn.rs:94`  | clone in `.or_else()` closure            |
| `Configuration/Proxy/Multi/Resolve/Fn.rs:101` | clone in `.or_else()` closure            |
| `Prefetch/Insert/Fn.rs:60`                    | clone in `.for_each()` closure           |
| `Prefetch/Read/Entry/Fn.rs:27`                | clone in `.map()` closure                |
| `Proxy/Handle/Proxy/Respond/Fn.rs:41`         | `serde_json::to_vec` twice in one body   |
| `PollWorker/Update/Fn.rs:102`                 | clone of a 256KB-capped accumulator (D7) |

### 9. Lock discipline

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p9 --examples 10
```

**1 total - 1 production (`aphrodite-hermes`), 0 test.** The probe resolves each `.lock()` site's
enclosing function, finds calls of the form `Module::Leaf::Fn(...)` after the lock, resolves the callee
through its module path, and asks whether the callee locks the same static. Exactly one site survives
that test, and it is D1: `Debug/Session/Record/Fn.rs:54` holds `SESSION_PARENTS` and calls
`Debug/Chain/Fn.rs:22`, which locks `SESSION_PARENTS` again. Every other `.lock()` site in the two
crates either scopes the guard in a block or calls nothing that locks the same static.

| Location                        | What is there                                                 |
| ------------------------------- | ------------------------------------------------------------- |
| `Debug/Session/Record/Fn.rs:54` | `static=SESSION_PARENTS callee=Debug/Chain/Fn.rs:22` (see D1) |

### 10. Unenforced bounds

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p10 --examples 10
```

**8 total - 7 production, 1 test, all in `aphrodite`.** Two are a `len() < N` test before an append:
one grows by a variable amount (the 256KB accumulator, D4) and one appends a single element, where the
bound does hold. Two collect a line into `Vec<char>`; four collect a whole input with
`lines().collect()`.

| Location                        | What is there                                                           |
| ------------------------------- | ----------------------------------------------------------------------- |
| `PollWorker/Update/Fn.rs:63`    | VARIABLE-GROWTH: `AccumulatedOutput.len() < 256 * 1024` then `push_str` |
| `Preview/Builder/Git.rs:29`     | single-push: `Paths.len() < 3` then one `push` (bound holds)            |
| `Marker/Format/Vars.rs:21`      | `let Chars:Vec<char> = Tmpl.chars().collect()`                          |
| `Preview/Text/SampleLong.rs:17` | `let Chars:Vec<char> = Line.chars().collect()`                          |
| `Preview/Builder/GitLog.rs:16`  | `let all:Vec<&str> = Inp.raw.lines().collect()`                         |
| `Retrieve/Page/Fn.rs:24`        | `let Lines:Vec<&str> = Content.lines().collect()`                       |
| `Template/Vars/Base/Fn.rs:24`   | `let Lines:Vec<&str> = Content.lines().collect()`                       |
| `Catalog/Test.rs:153`           | `Table.lines().collect()` (test)                                        |

### 11. Byte slicing

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p11 --examples 10
```

**33 total - 33 production (`aphrodite`), 0 test.** 19 are unguarded: neither the slice expression nor
the enclosing function carries a boundary helper. 11 sit in a function that does consult
`is_char_boundary`/`char_indices`/`TruncateCharSafe` somewhere, and 3 clamp the index with
`.min(...len())`. The guard test is per-function, so a `GUARDED` hit is a triage hint, not a proof: the
site still has to be read.

| Location                        | What is there                                        |
| ------------------------------- | ---------------------------------------------------- |
| `Marker/Format/Bracket.rs:24`   | `&Inner[..Colon]` - offset from a byte `find`        |
| `Preview/Builder/YAML.rs:19`    | `l.find(':').map(\|i\| &l[..i])` - no boundary check |
| `Preview/Line/Error.rs:45`      | `&t[j..i]` - two byte offsets                        |
| `Preview/Line/YAMLKey.rs:35`    | `&t[..Index]`                                        |
| `Proxy/State/Compress/Fn.rs:34` | `&bytes[..4096]` - bytes, no boundary needed         |
| `Resolve/Parse/Find.rs:34`      | `Content[AbsStart..AbsEnd]`                          |
| `Retrieve/Page/Fn.rs:63`        | `Lines[Start..End]` - line indices, not bytes        |
| `StructExtract/Rust.rs:65`      | `&AfterFn[..Paren]`                                  |
| `Template/Substitute/Fn.rs:19`  | `Out.push_str(&Rest[..Open])`                        |
| `Proxy/Body/Truncate/Fn.rs:15`  | GUARDED: `.rev().find(\|&b\| s.is_char_boundary(b))` |

### 12. Async shapes

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p12 --examples 10
```

**4 total - 3 production (`aphrodite`), 1 test.** Three `for` loops contain an `.await` in the body;
no `.await` appears inside a synchronous `for_each`/`try_for_each` closure anywhere in either crate,
which would not compile on a non-`Send` guard but is worth stating as a measured zero.

| Location                              | What is there                              |
| ------------------------------------- | ------------------------------------------ |
| `Proxy/Body/Capped/Struct.rs:32`      | `for` over body chunks with `.await`       |
| `Proxy/Compress/Chat/Fn.rs:44`        | `for` with `.await`                        |
| `Proxy/Handle/Proxy/Forward/Fn.rs:47` | retry `for` loop, one `.await` per attempt |
| `Retrieve/Test/Handler.rs:34`         | `for` with `.await` (test)                 |

### 13. Size outliers

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p13 --examples 100
```

**40 total - 38 production (35 `aphrodite`, 3 `aphrodite-hermes`), 2 test.** The full list, with line
counts and the natural split, is in **Separation candidates** below.

## Defect register

Each row was settled by reading the line at HEAD and by the probe named in it; no claim is taken from a
review. `D6` and `D7` are defects this sweep added to the five supplied. Verdicts are against the same
exported tree as the counts, so an uncommitted working-tree edit at one of these sites is not reflected
here.

| Id  | `path:line`                                                   | What breaks                                                                                                                                                                                                             | Probe                                                                                                                                                                                                                         | Consequence                                                                                                                                                                                                                                                                                | Verdict   |
| --- | ------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------- |
| D1  | `Debug/Session/Record/Fn.rs:52` → `:60`                       | `let mut map = SESSION_PARENTS...lock()` is a named binding, so the guard lives to the end of `Fn`; line 60 calls `Chain::Fn::Fn(session)`, which locks the same `OnceLock<Mutex<HashMap>>` (`Debug/Chain/Fn.rs:20-23`) | `python3 .hermes/tmp/audit/measure.py p9` → `Debug/Session/Record/Fn.rs:54 static=SESSION_PARENTS callee=Debug/Chain/Fn.rs:22`                                                                                                | `std::sync::Mutex` is not reentrant, so the thread blocks on its own lock and never returns. `pre_llm_call` hangs until Hermes abandons the hook worker, and every later `Chain::Fn::Fn` call blocks on the mutex the stuck thread still holds - the session tree stops resolving its root | CONFIRMED |
| D2  | `Boot/Listen/Watcher.rs:74` → `:112`                          | `let Received = Rx.recv().await.is_some();` with `false => {}`: a closed channel yields `Ready(None)` immediately, so the `loop` re-enters with no suspension point and no work                                         | read the scope: `Tx` is moved into the `notify` closure, which is stored in `Watcher`, a local of the same async block as the loop - no sender can be dropped while the loop runs                                             | The arm is dead code at HEAD, so nothing spins. It becomes a 100% CPU busy loop the moment the sender is dropped or cloned into a shorter-lived scope, which is the refactor the `false` arm invites                                                                                       | REFUTED   |
| D3  | `Proxy/Body/Stream/Fn.rs:27-28` vs `Proxy/Cache/Key/Fn.rs:20` | The stream fast path accepts the byte run `"stream":true` anywhere in the body; the cache key parses the top-level field. Two predicates, one body                                                                      | body `{"model":"m","messages":[],"stream_options":{"stream":true}}` → `windows(13)` match `True`, `v.get("stream")` `None`; body `{"model":"m","stream":false,"meta":{"stream": true}}` → fast path `True`, top level `False` | A request is forwarded on `StreamClient` (no total timeout) and is also cacheable, so a buffered JSON body can be stored and later replayed for a request the proxy classified as streaming - the case the cache key's own F3 comment forbids                                              | CONFIRMED |
| D4  | `PollWorker/Update/Fn.rs:63-71`                               | The cap is tested before the mutation it guards: `match task.AccumulatedOutput.len() < 256 * 1024 { true => { ... push_str(Output) } }`                                                                                 | read the arm - the test precedes one `push_str` of the whole poll payload, whose size is unbounded                                                                                                                            | One poll can append its entire output, so the accumulator exceeds the stated 256KB cap by up to one poll payload, and `LiveBytesArrived` (`:80`) then reports the over-cap length as the bytes arrived                                                                                     | CONFIRMED |
| D5  | `Preview/Builder/GitLog.rs:24-34`                             | The subject search rejects `commit ` lines instead of stopping at them, so `.find(...)` continues into the next commit's block                                                                                          | fixture replica of the same predicate over a subject-less commit prints `aaa1111 -> 'feat: add thing'` (the next commit's subject)                                                                                            | A commit with an empty message inherits the next commit's subject, so a log preview's first/last endpoint names the wrong change - and the endpoints are the whole preview                                                                                                                 | CONFIRMED |
| D6  | `PollWorker/Update/Fn.rs:97-126` (added)                      | `EffectiveExit` is parsed from one poll result but applied to every task whose status is `Running`: each is flushed, marked `Done`/`Failed` with that code and given a marker                                           | `MaximumBgTasks = 4` (`PollWorker/Task/Const.rs:11`) and `PollWorker/Status/Fn.rs:18` collects several running tasks, so more than one running task is the supported case                                                     | The first poll that carries an exit code finishes every running task: a second, still-running build is reported done, its accumulator is cleared and its marker records the other task's exit code                                                                                         | CONFIRMED |
| D7  | `PollWorker/Update/Fn.rs:102` (added)                         | `task.AccumulatedOutput.clone()` inside `filter_map`, once per running task per poll that carries an exit code                                                                                                          | same function: cap is 256KB (`:63`), up to four tasks live, and `:121` clears the original immediately after                                                                                                                  | Up to 1MB is copied per poll to move a string that is then cleared - the clone is the transfer mechanism, so a `take`/index-based handoff removes the copy. A cost, not a correctness break                                                                                                | CONFIRMED |

## Separation candidates

Every function over 80 lines, grouped by module, with the split that its own stage comments name.

```sh
AUDIT_ROOT=.hermes/tmp/audit/head python3 .hermes/tmp/audit/measure.py p13 --examples 100
```

### `crates/aphrodite` / Proxy (12)

| Function                              | Lines | Natural split                                                                                                                                          |
| ------------------------------------- | ----: | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Proxy/Handle/Proxy/Respond/Fn.rs:30` |   337 | one `match UpstreamResult` holding three bodies: SSE forward, buffered compress+store, error reply; plus header rebuild and the latency/request record |
| `Proxy/CCR/Create/Handle/Fn.rs:33`    |   194 | body decode (JSON vs octet-stream) / create / respond                                                                                                  |
| `Proxy/Compress/Chat/Fn.rs:27`        |   161 | budget curve (headroom → ratio) / transform / record                                                                                                   |
| `Proxy/State/Build/Fn.rs:43`          |   148 | client construction (bounded and stream clients) / state assembly                                                                                      |
| `Proxy/Relay/Execute/Fn.rs:31`        |   137 | one function per tool arm                                                                                                                              |
| `Proxy/Relay/Handle/Route/Fn.rs:28`   |    99 | validate / dispatch / reply                                                                                                                            |
| `Proxy/Handle/Proxy/Forward/Fn.rs:18` |    97 | attempt loop / error classification (timeout vs transport)                                                                                             |
| `Proxy/Handle/Proxy/Route/Fn.rs:21`   |    91 | dev request log / URL build / forward / cache lookup                                                                                                   |
| `Proxy/State/Method/Stats/Fn.rs:13`   |    91 | one function per stat family                                                                                                                           |
| `Proxy/Metadata/Rust/Fn.rs:4`         |    91 | one scanner per construct (fn, impl, struct, ...)                                                                                                      |
| `Proxy/Metadata/Python/Fn.rs:3`       |    90 | one scanner per construct (def, class, decorator, ...)                                                                                                 |
| `Proxy/CCR/Reload/Fn.rs:29`           |    87 | parse / apply / report                                                                                                                                 |

### `crates/aphrodite` / StructExtract (4)

| Function                     | Lines | Natural split                                                              |
| ---------------------------- | ----: | -------------------------------------------------------------------------- |
| `StructExtract/Rust.rs:26`   |   207 | one scanner per construct, plus the shared spent-budget walk as one helper |
| `StructExtract/JS.rs:23`     |   119 | same, JS constructs                                                        |
| `StructExtract/Go.rs:26`     |    97 | same, Go constructs                                                        |
| `StructExtract/Python.rs:25` |    83 | same, Python constructs                                                    |

### `crates/aphrodite` / Boot (3)

| Function                    | Lines | Natural split                                                                                                                                           |
| --------------------------- | ----: | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Boot/Route/Metrics.rs:42`  |   201 | one function per metric family: cache, latency buckets, ratio, inline CCR, tool relay, notify, upstream errors, CCR store, body bytes, upstream latency |
| `Boot/Listen/Watcher.rs:26` |    91 | watcher setup / event debounce / threshold apply                                                                                                        |
| `Boot/Fn.rs:22`             |    90 | `--version`/`--help` / `setup` subcommand / runtime build                                                                                               |

### `crates/aphrodite` / Hook (2)

| Function                            | Lines | Natural split                                                  |
| ----------------------------------- | ----: | -------------------------------------------------------------- |
| `Hook/ToolResult/Inner/Fn.rs:25`    |   136 | telemetry record (must run first) / compress decision / marker |
| `Hook/Terminal/ChainSplit/Fn.rs:26` |    83 | split event record / dev metrics / compress decision           |

### `crates/aphrodite` / Preview (2)

| Function                     | Lines | Natural split                                             |
| ---------------------------- | ----: | --------------------------------------------------------- |
| `Preview/Builder/Test.rs:16` |   130 | one arm per runner output shape (cargo, pytest, bare log) |
| `Preview/Builder/Ls.rs:14`   |    83 | line classification / tally / render                      |

### `crates/aphrodite` / Session (2)

| Function                   | Lines | Natural split                                                                    |
| -------------------------- | ----: | -------------------------------------------------------------------------------- |
| `Session/Catalog/Fn.rs:23` |   129 | one section per source: delta markers, archived turns, delta files, poll workers |
| `Session/Engine/Fn.rs:42`  |   118 | gates (engine off, threshold) / fill ratio / middle slice                        |

### `crates/aphrodite` / Setup (2)

| Function                 | Lines | Natural split                                                  |
| ------------------------ | ----: | -------------------------------------------------------------- |
| `Setup/Run/Entry.rs:54`  |   172 | runtime-home resolve / one function per install step           |
| `Setup/Dylib/Copy.rs:42` |   132 | per-dylib copy (core is best-effort, hermes required) / verify |

### `crates/aphrodite` / one each

| Function                                                          | Lines | Natural split                                                       |
| ----------------------------------------------------------------- | ----: | ------------------------------------------------------------------- |
| `ABI/Dispatch.rs:24`                                              |   140 | per-host-call span / routing / handle lookup / result               |
| `ConfigurationLoader/Configuration/Apply/Section/Directive.rs:15` |   125 | directives section apply (one function)                             |
| `PollWorker/Update/Fn.rs:22`                                      |   121 | phase 1 (accumulate) / phase 2 (complete + marker) - see D4, D6, D7 |
| `Configuration/Proxy/Multi/Resolve/Fn.rs:24`                      |   113 | env / file / default resolve, one per layer                         |
| `Flow/TurnContext/Collect/Fn.rs:30`                               |   107 | one section per always-survive / conditional group                  |
| `Prefetch/Insert/Fn.rs:20`                                        |   105 | outcome insert / report                                             |
| `Catalog/Build/Fn.rs:12`                                          |    96 | group by type / render                                              |
| `Directive/Action.rs:25`                                          |    88 | one arm per action                                                  |
| `Retrieve/Handle/Fn.rs:26`                                        |   198 | inline store lookup / page window / reply                           |

### `crates/aphrodite-hermes`

| Function                              | Lines | Natural split                                       |
| ------------------------------------- | ----: | --------------------------------------------------- |
| `Tool/Unwrap/Fn.rs:27`                |   134 | one arm per wrapper shape (terminal, search, JSON)  |
| `Hook/Call/Dispatch/PreTool/Fn.rs:21` |   106 | dev log / auto-background intercept / compress path |
| `Tool/Unwrap/Search/Fn.rs:23`         |    81 | match line render / count / preview                 |
| `Tool/Test/Unwrap.rs:13`              |   115 | table test - one case per row (test)                |

## Clean modules

Modules with zero production findings, so a rewrite package can skip them:

- `crates/aphrodite`: `(root)` - `Library.rs` and `Main.rs`, 119 lines, no hit in any measurement.
- `crates/aphrodite-hermes`: `Schema` (35 files, 652 lines - the three findings there are all in
  `Schema/Test/Shape.rs`), `Health` (5 files, 90 lines), `Guard` (2 files, 69 lines), `Bin` (1 file, 32
  lines), `(root)` (1 file, 75 lines).

Every other module has at least one production finding; the two with the highest density per line are
`StructExtract` (61 findings over 870 lines) and `Boot` (54 over 1,429), both dominated by the
no-op-arm shape of measurement 2.
