---
name: aphrodite-auto-expand-testing
description: "Use when debugging why CCR markers appear raw in the LLM view. Inert-config record for auto-expand, C-001 resolution, live-read thresholds, retrieve-first rule."
version: 3.0.0
platforms: [macos]
tags: [aphrodite, ccr, auto-expand, testing, inert-config]
status: active
scope:
    repositories:
        - PlayForm/Aphrodite
        - PlayForm/Aphrodite-Hermes
    branches:
        - Development
    runtime_modes:
        - source
        - installed
owns:
    - The inert-configuration record for auto-expand (status: inactive)
    - C-001 resolution (auto-expand nonfunctional until runtime-proved)
    - The live-read threshold rule and the T-1/T/T+1 transition test procedure
depends_on:
    - aphrodite-boundaries
    - aphrodite-orientation
supersedes: []
verification:
    source_of_truth:
        - crates/aphrodite/src/proxy.rs (config endpoint serialization, no-consumer comment)
        - plugins/aphrodite/__init__.py (no AUTO_EXPAND handling)
        - ~/.hermes/aphrodite/aphrodite.toml (live config values)
mutation_level: read-only
---

# Aphrodite Auto-Expand Testing

Protocol for understanding when `<<<CCR:...>>>` markers appear raw in the
LLM's view - and for keeping the auto-expand configuration honest about being
**inert**. This skill is the canonical owner of contradiction C-001 and of the
inert-configuration record below. It is `mutation_level: read-only`: it never
mutates configuration; it reads live state and records observations.

## C-001 resolution (canonical owner)

The contradiction register (`.hermes/governance/CONTRADICTION-REGISTER.md`,
row C-001) records the conflict: "Enable auto-expand" (development lessons) vs
"no active consumer" (source). **Decision: treat auto-expand as nonfunctional
until runtime-proved.** Auto-expand is configuration observability only; it is
never a debugging guarantee and never a remediation for raw CCR markers. The
verification for this decision is the fixture test defined below (payload
above threshold, before/after). When a CCR marker appears, retrieve it through
the canonical retrieval route; validate any claimed auto-expansion behavior
with an explicit before/after test before believing it.

## Inert configuration record: auto-expand

The record below is the exact, canonical statement. "Inert configuration" is a
first-class status: a field may parse and echo in status responses while
having no operational effect. Do not "fix" raw markers by reintroducing a
configuration field - the only working path is retrieval.

## Retired configuration: auto-expand

**Status:** Parsed but inactive.

**Accepted inputs:** TOML keys may parse; environment variables may be present.

**Operational effect:** None unless a source-derived consumer is reintroduced.

**User instruction:** Do not configure this as a remediation for raw CCR markers.

**Replacement behavior:** Retrieve markers through the canonical resolver.

**Reactivation gate:** A pull request must add a real consumer, integration
tests, runtime status evidence, and update this record from `inactive` to
`active`.

## Ground truth (source-derived; re-verify in current source)

**Confidence:** source-derived.
**Verify:** grep `auto_expand` in `crates/aphrodite/src/` and
`AUTO_EXPAND` in `plugins/aphrodite/__init__.py` at the current commit.
**If different:** a real consumer now exists - run the reactivation gate and
update this record to `active`.

- The Rust proxy serializes `auto_expand` / `auto_expand_limit` in its config
  endpoint (`crates/aphrodite/src/proxy.rs`), but **no consumer reads them** -
  the source comment at that site marks `engine_threshold_pct`, `catalog_mode`,
  and `auto_expand*` as having no consumer. A repo-wide grep finds the keys
  only in `proxy.rs` and the config struct (`config/proxy.rs`): serialization
  and parsing sites only.
- The plugin (`plugins/aphrodite/__init__.py`) has **no** `AUTO_EXPAND`
  handling (grep count: 0); the `pre_llm_call` hook served by the dylib
  (`crates/aphrodite-hermes/src/lib.rs`) injects a catalog/guidance context -
  it does not expand markers.
- `APHRODITE_NO_AUTO_EXPAND` and `APHRODITE_AUTO_EXPAND` both have **zero
  consumers** in source - never use either; they silently change nothing.
- Live `~/.hermes/aphrodite/aphrodite.toml` may still carry `auto_expand` and
  `auto_expand_limit` keys - they are inert leftover keys (parsed config
  fields, echoed in the status response only).

Consequence: markers appear raw unless the context engine already resolved
them. Whenever you SEE a marker, `aphrodite_retrieve(hash)` it immediately -
retrieval is the only path that guarantees content.

## Thresholds must be live-read

> Never put a threshold literal in a step's expected outcome unless the step
> first reads the active configuration and prints the value it is about to
> test.

Thresholds such as the engine percentage and the terminal-output size are
configuration-dependent. A literal in an expected outcome both goes stale and
hides "parsed but ignored" behavior. Use the T-1/T/T+1 transition procedure:

1. Read the active terminal threshold `T` from live config
   (`[compression] terminal_threshold`, env override
   `APHRODITE_TERMINAL_THRESHOLD`) and print it.
2. Generate payload at `T - 1` bytes.
3. Generate payload at `T` bytes.
4. Generate payload at `T + 1` bytes.
5. Verify each observed transition.
6. Record the raw config source, environment override presence, and binary
   version.

This detects both off-by-one errors and "configuration parsed but ignored"
behavior.

## The Three Layers

1. **Proxy response compression** (Rust, token listener - the port is a
   config property, read it from the running proxy / `aphrodite.toml` `ports`
   (default token :9798); verify live, never assume) - compresses provider
   RESPONSE messages. Produces `<<<CCR:...>>>` in model responses. Always
   active.
2. **Context engine** - compresses MIDDLE messages in conversation_history
   into a CCR marker when the threshold is reached. Fires on turn 2+; the LLM
   sees markers and polls via `aphrodite_retrieve()`.
3. **pre_llm catalog injection** - the dylib's `pre_llm_call` arm injects a
   catalog/guidance context string. No marker expansion happens here.

## Engine knobs (source-derived; re-verify in source)

**Confidence:** source-derived - precedence is `config_loader.rs` (env > TOML

> default); read current defaults from source before citing them.

| Knob                       | Env var                          | TOML key                           | Default |
| -------------------------- | -------------------------------- | ---------------------------------- | ------- |
| Compression threshold %    | `APHRODITE_ENGINE_THRESHOLD_PCT` | `compression.engine_threshold_pct` | 45      |
| Min messages before engine | `APHRODITE_ENGINE_MIN_MSGS`      | `compression.engine_min_msgs`      | 8       |
| Protect first N            | `APHRODITE_ENGINE_PROTECT_FIRST` | `compression.engine_protect_first` | 2       |
| Protect last N             | `APHRODITE_ENGINE_PROTECT_LAST`  | `compression.engine_protect_last`  | 5       |

Runtime-derived (read live, never assume): the live TOML sets
`engine_threshold_pct = 100` (100% = effectively disables CCR compression of
tool output) - do not expect tool-output markers by default; markers come from
terminal output, the token proxy, and cross-session content instead. The
plugin registers the context engine only when `APHRODITE_CONTEXT_ENGINE=1` is
set; the dylib side additionally honors `compression.context_engine` (default
true).

## Protocol

### Step 1: Read current config (live-read)

```bash
grep -n "auto_expand\|engine_threshold\|context_engine\|terminal_threshold" ~/.hermes/aphrodite/aphrodite.toml
```

Print the values you are about to test; a threshold you have not printed is a
threshold you may not assert on.

### Step 2: Force engine compression to observe markers

```bash
APHRODITE_ENGINE_THRESHOLD_PCT=1 \
	APHRODITE_ENGINE_MIN_MSGS=4 \
	APHRODITE_ENGINE_PROTECT_FIRST=1 \
	APHRODITE_ENGINE_PROTECT_LAST=1 \
	hermes
```

Run several tool calls. After turn 2+, the context engine compresses middle
tool results - the LLM sees `<<<CCR:hash|context|N>>>` and must poll with
`aphrodite_retrieve(hash)`. State the threshold you are testing (live-read
above) and record the binary version.

### Step 3: Verify retrieval always works

`aphrodite_retrieve(hash="<hash>")` returns the full original content
regardless of config.

## Expected Behavior Matrix (live-read)

Each row's threshold is read live at probe time; the numbers below are the
shipped-default snapshot, not constants.

| Config                      | What LLM sees                 | Retrieve returns |
| --------------------------- | ----------------------------- | ---------------- |
| Live TOML (threshold 100%)  | Full content (no markers)     | Full content     |
| Engine threshold forced low | `<<<CCR:hash\|context\|N>>>`  | Full content     |
| Terminal output above T     | `<<<CCR:hash\|terminal\|N>>>` | Full content     |

The terminal threshold is a config property (`[compression]
terminal_threshold`, env-overridable `APHRODITE_TERMINAL_THRESHOLD`) - read it
from the live config at probe time, never hardcode.

## Fixture test (C-001 verification)

The verification for the C-001 decision. A claim of auto-expansion is proved
only by a controlled before/after test, never by configuration prose:

1. Create a known payload larger than the compression threshold.
2. Read it once under the proposed auto-expand setting.
3. Record whether the response is inline content, a valid marker, or a
   malformed result.
4. If it is a marker, resolve it once through the canonical retrieval tool.
5. Compare bytes or normalized text with the source payload.
6. Record the runtime binary version and configuration source.

Use the deterministic fixture suite owned by `aphrodite-benchmarking` (and its
recorded `fixtures_version`). With the current source the expected observation
is: a marker appears and requires canonical retrieval - proving the
configuration is inert. The moment the before/after test shows markers
resolving without retrieval, the reactivation gate applies.

## Pitfalls

- never chase `auto_expand` config to explain raw markers - the keys have no
  consumer in the current source; the retrieve-first rule is the answer
- never use `APHRODITE_NO_AUTO_EXPAND` or `APHRODITE_AUTO_EXPAND` - neither
  has a consumer; they silently change nothing
- never configure auto-expand as remediation for raw CCR markers - that is the
  inert-config user instruction, and the anti-feature record forbids it
- never test with a single-turn session (`hermes -z`) - the context engine
  needs turn 2+
- don't confuse the layers - the proxy compresses provider responses (token
  listener port is a config property, default :9798 - read live, don't
  assume); the engine compresses conversation middle messages
- never hardcode a threshold in an expected outcome - live-read the active
  config and print the value first
- protected first/last messages (`engine_protect_first` /
  `engine_protect_last`) stay raw regardless of threshold

## Local claim-to-test matrix

| Claim                             | Evidence source                    | Test                                                      | Pass condition                                    | Failure response                                           |
| --------------------------------- | ---------------------------------- | --------------------------------------------------------- | ------------------------------------------------- | ---------------------------------------------------------- |
| `auto_expand` has no consumer     | `proxy.rs`, plugin source          | Grep `auto_expand` in `crates/`, `AUTO_EXPAND` in plugin  | Serialization/struct sites only; zero plugin hits | A consumer exists → run the reactivation gate              |
| Env vars change nothing           | Source consumer scan               | Run a round trip with and without `APHRODITE_AUTO_EXPAND` | Behavior identical both ways                      | Consumer introduced → update record to `active`            |
| Threshold transitions are correct | Live config read                   | T-1/T/T+1 procedure with printed `T`                      | Transition observed at the live `T`               | Threshold literal drifted → fix the procedure              |
| Retrieval always resolves         | `aphrodite_retrieve`               | Compress known payload, retrieve once                     | Original normalized content                       | Resolver failure → escalate (boundaries: fail-open policy) |
| C-001 fixture test                | Fixture suite (benchmarking owner) | Before/after test with payload above threshold            | Marker appears and requires canonical retrieval   | Auto-expand proved functional → reactivate the record      |
