"""RELEASE_FLOW_TASK"""

from .types import Conversation, ToolCall, Turn

# ═══════════════════════════════════════════════════════════════════════════════
RELEASE_FLOW_TASK = Conversation(
    name="release_flow_task",
    description="Run the Development → Current release ritual - version parity, gates, sync commit, tag, restore",
    system_prompt="""You are a release engineering assistant for a two-branch project (Development = workshop, Current = distributed branch). Releases are snapshot transplants, never merges: prepare the bump on Development, run the gates, sync the plugin submodule first, then the parent, restore protected paths (.gitmodules, .github/workflows, the submodule gitlink), float the submodule to the Current tip, tag on Current only, and end with the working copy back on Development. Verify every step with machine-checkable exit codes and never tag on Development.""",
    turns=[
        # Turn 1: User requests the release ritual
        Turn(
            role="user",
            content="""We need to cut the next release: sync Development → Current for Aphrodite v1.4.3 (plugin v2.1.3). The repo is at `/tmp/aphrodite-release/` with the plugin submodule at `plugins/aphrodite`. Walk the full ritual: verify version parity, run the gate set, sync the plugin first, then the parent with the protected-path restore and submodule float, tag on Current only, and end with the working copy back on Development. Keep commands machine-checkable - echo exit codes so we can verify each step.""",
        ),
        # Turn 2: Assistant checks version parity and tree state
        Turn(
            role="assistant",
            content="Starting the ritual. First verify version parity and a clean tree on Development.",
            tool_calls=[
                ToolCall(
                    id="call_r1",
                    name="terminal",
                    arguments='{"command": "cd /tmp/aphrodite-release && git status --short 2>&1; echo EXIT:$?"}',
                ),
                ToolCall(
                    id="call_r2",
                    name="terminal",
                    arguments='{"command": "cd /tmp/aphrodite-release && grep -n \\"version\\" crates/aphrodite/Cargo.toml crates/aphrodite-hermes/Cargo.toml plugins/aphrodite/plugin.yaml plugins/aphrodite/BINARY_VERSION package.json 2>/dev/null; echo EXIT:$?"}',
                ),
                ToolCall(
                    id="call_r3",
                    name="terminal",
                    arguments='{"command": "cd /tmp/aphrodite-release && git log --oneline -3; echo EXIT:$?"}',
                ),
            ],
        ),
        # Turn 3: Tool results - clean tree, version parity, recent commits
        Turn(
            role="tool",
            tool_call_id="call_r1",
            content="""On branch Development
Your branch is up to date with 'Source/Development'.

nothing to commit, working tree clean

EXIT:0
""",
        ),
        Turn(
            role="tool",
            tool_call_id="call_r2",
            content="""crates/aphrodite/Cargo.toml:1:[package]
crates/aphrodite/Cargo.toml:3:version = "1.4.3"
crates/aphrodite-hermes/Cargo.toml:3:version = "1.4.3"
crates/aphrodite-hermes/Cargo.toml:18:aphrodite = { version = "1.4.3", path = "../aphrodite" }
plugins/aphrodite/plugin.yaml:7:version: 2.1.3
plugins/aphrodite/BINARY_VERSION:1:2.1.3
package.json:3:"version": "1.4.3"

Version parity OK: binary 1.4.3, plugin 2.1.3 (monotonic vs 1.4.2 / 2.1.2 on Current)
EXIT:0
""",
        ),
        Turn(
            role="tool",
            tool_call_id="call_r3",
            content="""a1b2c3d (HEAD -> Development) chore: bump v1.4.3 (binary) + v2.1.3 (plugin)
e4f5a6b feat: add classifier pre-filter for clean outputs
9c8d7e6 fix: poll worker reconnects on proxy restart
EXIT:0
""",
        ),
        # Turn 4: Assistant runs the gate set
        Turn(
            role="assistant",
            content="Versions match and the tree is clean - the bump commit is on Development. Running the gates: clippy with -D warnings, advisory scan, and the import check.",
            tool_calls=[
                ToolCall(
                    id="call_r4",
                    name="terminal",
                    arguments='{"command": "cd /tmp/aphrodite-release && cargo clippy -p aphrodite -p aphrodite-hermes -- -D warnings 2>&1 | tail -40; echo EXIT:${PIPESTATUS[0]}"}',
                ),
                ToolCall(
                    id="call_r5",
                    name="terminal",
                    arguments='{"command": "cd /tmp/aphrodite-release && cargo deny check advisories 2>&1; echo EXIT:$?"}',
                ),
                ToolCall(
                    id="call_r6",
                    name="terminal",
                    arguments='{"command": "cd /tmp/aphrodite-release && python3 -c \\"import aphrodite\\" 2>&1; echo EXIT:$?"}',
                ),
            ],
        ),
        # Turn 5: Tool results - large clippy build output, deny scan, import check
        Turn(
            role="tool",
            tool_call_id="call_r4",
            content="""   Compiling proc-macro2 v1.0.86
   Compiling quote v1.0.36
   Compiling syn v2.0.72
   Compiling unicode-ident v1.0.12
   Compiling autocfg v1.3.0
   Compiling libc v0.2.158
   Compiling cfg-if v1.0.0
   Compiling version_check v0.9.5
   Compiling once_cell v1.20.1
   Compiling lazy_static v1.5.0
   Compiling memchr v2.7.4
   Compiling regex-syntax v0.8.4
   Compiling regex-automata v0.4.7
   Compiling regex v1.10.5
   Compiling serde v1.0.228
   Compiling serde_derive v1.0.228
   Compiling serde_json v1.0.150
   Compiling itoa v1.0.11
   Compiling ryu v1.0.18
   Compiling indexmap v2.5.0
   Compiling hashbrown v0.15.2
   Compiling ahash v0.8.11
   Compiling zerocopy v0.7.35
   Compiling byteorder v1.5.0
   Compiling subtle v2.6.1
   Compiling zeroize v1.8.1
   Compiling digest v0.10.7
   Compiling crypto-common v0.1.6
   Compiling generic-array v0.14.7
   Compiling typenum v1.17.0
   Compiling cpufeatures v0.2.14
   Compiling ppv-lite86 v0.2.20
   Compiling getrandom v0.2.15
   Compiling rand_core v0.6.4
   Compiling rand_chacha v0.3.1
   Compiling rand v0.8.5
   Compiling blake3 v1.5.4
   Compiling constant_time_eq v0.3.1
   Compiling arrayref v0.3.9
   Compiling arrayvec v0.7.6
   Compiling hex v0.4.3
   Compiling base64 v0.22.1
   Compiling sha2 v0.10.8
   Compiling hmac v0.12.1
   Compiling num-bigint v0.4.6
   Compiling num-traits v0.2.19
   Compiling num-integer v0.1.46
   Compiling chrono v0.4.39
   Compiling time v0.3.36
   Compiling time-core v0.1.2
   Compiling uuid v1.11.0
   Compiling bytes v1.7.1
   Compiling pin-project-lite v0.2.15
   Compiling pin-project v1.1.7
   Compiling futures-core v0.3.31
   Compiling futures-channel v0.3.31
   Compiling futures-sink v0.3.31
   Compiling futures-util v0.3.31
   Compiling futures v0.3.31
   Compiling parking_lot_core v0.9.10
   Compiling parking_lot v0.12.3
   Compiling mio v1.0.2
   Compiling socket2 v0.5.8
   Compiling tokio v1.52.3
   Compiling tokio-util v0.7.13
   Compiling async-trait v0.1.83
   Compiling http v1.2.0
   Compiling http-body v0.4.6
   Compiling httparse v1.9.4
   Compiling h2 v0.4.6
   Compiling tower v0.5.1
   Compiling tower-layer v0.3.3
   Compiling tower-http v0.6.2
   Compiling axum v0.8.9
   Compiling axum-core v0.5.0
   Compiling mime v0.3.17
   Compiling percent-encoding v2.3.1
   Compiling url v2.5.2
   Compiling tracing v0.1.40
   Compiling tracing-core v0.1.32
   Compiling tracing-subscriber v0.3.18
   Compiling clap v4.5.20
   Compiling clap_builder v4.5.20
   Compiling clap_derive v4.5.18
   Compiling anyhow v1.0.89
   Compiling thiserror v1.0.64
   Compiling dashmap v6.1.0
   Compiling sqlx v0.8.3
   Compiling ed25519-dalek v2.1.1
   Compiling curve25519-dalek v4.1.3
   Compiling rustls v0.23.16
   Compiling rustls-pki-types v1.8.0
   Compiling x509-parser v0.16.0
   Checking serde v1.0.228
   Checking serde_json v1.0.150
   Checking tokio v1.52.3
   Checking hyper v1.8.1
   Checking axum v0.8.9
   Checking tower v0.5.1
   Checking tower-http v0.6.2
   Checking tracing v0.1.40
   Checking tracing-subscriber v0.3.18
   Checking clap v4.5.20
   Checking blake3 v1.5.4
   Checking sqlx v0.8.3
   Checking uuid v1.11.0
   Checking chrono v0.4.39
   Checking regex v1.10.5
   Checking anyhow v1.0.89
   Checking thiserror v1.0.64
   Checking reqwest v0.12.9
   Checking mime v0.3.17
   Checking h2 v0.4.6
   Checking http-body-util v0.1.2
   Checking serde_urlencoded v0.7.1
   Checking form_urlencoded v1.2.1
   Checking tokio-stream v0.1.16
   Checking tokio-util v0.7.13
   Checking futures-io v0.3.31
   Checking slab v0.4.9
   Checking mio v1.0.2
   Checking socket2 v0.5.8
   Checking parking_lot v0.12.3
   Checking smallvec v1.13.2
   Checking once_cell v1.20.1
   Checking dashmap v6.1.0
   Checking tracing-appender v0.2.3
   Checking rustls v0.23.16
   Checking ed25519-dalek v2.1.1
   Checking curve25519-dalek v4.1.3
   Checking aphrodite-core v1.4.3 (/tmp/aphrodite-release/crates/aphrodite-core)
   Checking aphrodite v1.4.3 (/tmp/aphrodite-release/crates/aphrodite)
   Checking aphrodite-hermes v1.4.3 (/tmp/aphrodite-release/crates/aphrodite-hermes)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 41.2s

clippy gate: PASS (-D warnings, 0 warnings emitted)
EXIT:0
""",
        ),
        Turn(
            role="tool",
            tool_call_id="call_r5",
            content="""[fetch] advisory DB: crates.io-advisories (frozen)
[fetch] lock file: /tmp/aphrodite-release/Cargo.lock
[analyze] Scanning 214 crates for vulnerabilities...
[analyze] Found 0 vulnerabilities
[analyze] 0 ignored, 0 error-level, 0 warning-level
deny gate: PASS (0 advisories)
EXIT:0
""",
        ),
        Turn(
            role="tool",
            tool_call_id="call_r6",
            content="""aphrodite loaded from /tmp/aphrodite-release/target/release
import gate: PASS
EXIT:0
""",
        ),
        # Turn 6: User approves the sync
        Turn(
            role="user",
            content="All gates are green. Proceed with the sync: plugin first, then the parent. Remember the protected-path restore and submodule float, and keep the tag on Current only.",
        ),
        # Turn 7: Assistant syncs the plugin submodule (S-Dev → S-Current)
        Turn(
            role="assistant",
            content="Syncing the plugin first - the parent's gitlink must reference the released plugin.",
            tool_calls=[
                ToolCall(
                    id="call_r7",
                    name="terminal",
                    arguments='{"command": "cd /tmp/aphrodite-release/plugins/aphrodite && git checkout Current 2>&1 && git merge --squash Development 2>&1 && git commit -m \\"release: sync v2.1.3 from Development\\" 2>&1 && git tag v2.1.3 2>&1 && git push Source Current --tags 2>&1; echo EXIT:$?"}',
                ),
            ],
        ),
        # Turn 8: Tool result - plugin sync output
        Turn(
            role="tool",
            tool_call_id="call_r7",
            content="""Switched to branch 'Current'
Your branch is up to date with 'Source/Current'.
Squash commit -- not updating HEAD
 Automatic merge went well; stopped before committing as requested
 12 files changed, 340 insertions(+), 52 deletions(-)
[Current 5f6e7d8] release: sync v2.1.3 from Development
 12 files changed, 340 insertions(+), 52 deletions(-)
To /tmp/mirrors/aphrodite-hermes.git
 * [new tag]         v2.1.3 -> v2.1.3
 * [new branch]      Current -> Current

plugin sync: PASS (v2.1.3 tagged on S-Current)
EXIT:0
""",
        ),
        # Turn 9: Assistant syncs the parent (P-Dev → P-Current) + tag
        Turn(
            role="assistant",
            content="Plugin is on Current at v2.1.3. Now the parent sync: clean-tree check, squash Development into Current, restore protected paths, float the submodule to S-Current, verify, commit, tag, push, and return to Development.",
            tool_calls=[
                ToolCall(
                    id="call_r8",
                    name="terminal",
                    arguments='{"command": "cd /tmp/aphrodite-release && test -z \\"$(git status --porcelain)\\" && echo TREE_CLEAN && git checkout Current 2>&1 && git pull --ff-only Source Current 2>&1 && git merge --squash Development 2>&1 && git checkout HEAD -- .gitmodules .github/workflows plugins/aphrodite && git -C plugins/aphrodite fetch Source Current 2>&1 && git -C plugins/aphrodite checkout -B Current Source/Current 2>&1 && git add plugins/aphrodite && test -z \\"$(git diff --cached --name-only -- .gitmodules .github/workflows)\\" && echo PROTECTED_PATHS_RESTORED && git commit -m \\"release: sync v1.4.3 from Development\\" 2>&1 && git tag Aphrodite/v1.4.3 2>&1 && git push Source Current --tags 2>&1 && git checkout Development 2>&1; echo EXIT:$?"}',
                ),
            ],
        ),
        # Turn 10: Tool result - large parent sync output with diffstat
        Turn(
            role="tool",
            tool_call_id="call_r8",
            content="""TREE_CLEAN
Switched to branch 'Current'
Your branch is up to date with 'Source/Current'.
Squash commit -- not updating HEAD
 Automatic merge went well; stopped before committing as requested
 55 files changed, 1682 insertions(+), 141 deletions(-)

 .gitmodules                                          |  2 +-
 .github/workflows/build.yml                          |  4 +-
 .github/workflows/release.yml                        |  2 +-
 README.md                                            | 12 ++++--
 CHANGELOG.md                                         |  8 ++++
 docs/release-notes.md                                | 15 ++++++++
 package.json                                         |  2 +-
 crates/aphrodite/Cargo.toml                          |  3 +-
 crates/aphrodite-core/Cargo.toml                     |  3 +-
 crates/aphrodite-hermes/Cargo.toml                   |  5 ++--
 crates/aphrodite-hermes/src/lib.rs                    | 24 +++++++----
 crates/aphrodite/src/main.rs                          |  6 ++--
 src/compression/classifier.rs                         | 28 ++++++++++++-
 src/compression/engine.rs                             | 18 ++++++---
 src/compression/preview.rs                            | 12 ++++---
 src/compression/marker.rs                             |  4 +-
 src/compression/store.rs                              |  9 +++--
 src/compression/retrieve.rs                           |  7 ++--
 src/compression/catalog.rs                            |  5 ++-
 src/compression/inline.rs                             |  3 +-
 src/config.rs                                        | 21 +++++++++--
 src/session.rs                                       | 10 +++++-
 src/flow.rs                                          |  8 +++--
 src/hooks.rs                                         |  6 ++--
 src/poll_worker.rs                                   |  9 +++--
 src/state.rs                                         |  4 +-
 src/directives.rs                                    |  5 ++-
 src/resolve.rs                                       |  6 ++--
 src/setup.rs                                         |  4 +-
 src/tools/aphrodite_catalog.rs                       |  3 +-
 src/tools/aphrodite_prefetch.rs                      |  7 ++--
 src/tools/aphrodite_stats.rs                         |  3 +-
 src/tools/aphrodite_retrieve.rs                      |  5 ++-
 src/tools/aphrodite_search.rs                        |  4 +-
 src/tools/aphrodite_reclassify.rs                    |  6 ++--
 src/tools/aphrodite_directive.rs                     |  3 +-
 src/proxy.rs                                        | 34 +++++++++++++---
 src/lib.rs                                          | 16 ++++++---
 plugins/aphrodite/BINARY_VERSION                     |  2 +-
 plugins/aphrodite/plugin.yaml                        |  3 +-
 plugins/aphrodite/install_message.md                 |  6 ++--
 tests/integration_proxy.rs                           | 42 +++++++++++++++++--
 tests/integration_ccr.rs                             | 25 ++++++++++--
 tests/unit_classifier.rs                             | 14 +++++---
 tests/unit_preview.rs                                | 10 ++++--
 src/handlers/openai.rs                               | 12 ++++++--
 src/handlers/anthropic.rs                            | 8 ++++---
 src/handlers/gemini.rs                               | 5 ++--
 src/handlers/streaming.rs                            | 14 ++++++---
 src/handlers/batch.rs                                | 4 +-
 src/tools/aphrodite_compress.rs                      | 6 ++--
 src/tools/aphrodite_diff.rs                          | 3 +-
 src/tools/aphrodite_files.rs                         | 3 +-
 src/tools/aphrodite_rebuild.rs                       | 3 +-
 src/tools/aphrodite_test.rs                          | 4 +-
 plugins/aphrodite (submodule)                        | 1 +
 create mode 100644 docs/release-notes.md
 delete mode 100644 docs/archive/notes-1.3.4.md

Restored from HEAD (Current-owned): .gitmodules, .github/workflows, plugins/aphrodite
Fetching Source Current
Branch 'Current' set up to track 'Source/Current'.
HEAD is now at 5f6e7d8 release: sync v2.1.3 from Development (S-Current tip)
PROTECTED_PATHS_RESTORED
[Current 9a8b7c6] release: sync v1.4.3 from Development
 55 files changed, 1682 insertions(+), 141 deletions(-)
To /tmp/mirrors/aphrodite.git
 * [new tag]         Aphrodite/v1.4.3 -> Aphrodite/v1.4.3
 * [new branch]      Current -> Current
Switched to branch 'Development'
Your branch is up to date with 'Source/Development'.

parent sync: PASS (Aphrodite/v1.4.3 tagged on Current, worktree back on Development)
EXIT:0
""",
        ),
        # Turn 11: Assistant runs final invariant verification
        Turn(
            role="assistant",
            content="Sync complete. Final verification pass - branch, submodule pin, and tag placement.",
            tool_calls=[
                ToolCall(
                    id="call_r9",
                    name="terminal",
                    arguments='{"command": "cd /tmp/aphrodite-release && git branch --show-current && git submodule status && git -C plugins/aphrodite describe --tags --exact-match HEAD 2>/dev/null; git describe --tags 2>/dev/null || echo NO_TAGS_ON_DEVELOPMENT; echo EXIT:$?"}',
                ),
            ],
        ),
        # Turn 12: Tool result - verification output
        Turn(
            role="tool",
            tool_call_id="call_r9",
            content="""Development
 5f6e7d8... plugins/aphrodite (v2.1.3)
v2.1.3
NO_TAGS_ON_DEVELOPMENT

verify: PASS (worktree on Development, submodule pinned with no '+', tags live on Current only)
EXIT:0
""",
        ),
        # Turn 13: Final release summary
        Turn(
            role="assistant",
            content="""The Development → Current release ritual is complete. Summary:

1. **Version parity** - binary 1.4.3 (Cargo.toml, package.json), plugin 2.1.3 (plugin.yaml, BINARY_VERSION); monotonic vs 1.4.2/2.1.2 on Current.
2. **Gates** - clippy `-D warnings` PASS, `cargo deny check advisories` PASS (0 of 214 crates vulnerable), import check PASS.
3. **Plugin sync** (S-Dev → S-Current) - squash of Development into Current, commit `5f6e7d8 release: sync v2.1.3`, tagged `v2.1.3`, pushed.
4. **Parent sync** (P-Dev → P-Current) - clean tree confirmed, Development squashed into Current (55 files), protected paths (`.gitmodules`, `.github/workflows`, submodule gitlink) restored from HEAD, submodule floated to S-Current tip, commit `9a8b7c6 release: sync v1.4.3`, tagged `Aphrodite/v1.4.3`, pushed.
5. **Invariants** - worktree on Development (I10), submodule status clean with no `+` (I2), tags on Current only (V5), versions monotonic (I8).

Ready for the GitHub release from the `Aphrodite/v1.4.3` tag.""",
        ),
    ],
)
