# Aphrodite Governance - CONTRADICTION-REGISTER.md

Living contradiction register. Every removed, superseded, or conditional rule
in the Aphrodite skill system is logged here with its decision and canonical
owner. **This register is living**: agents in the 2026-09-18 rewrite wave add
rows as they find conflicts (C-005 onward); nobody deletes or rewrites a
seeded row - a changed decision appends a new row that supersedes it.

Resolving a contradiction is not "append another exception": update the
canonical instruction, mark the obsolete one retired, and record both in this
register.

## Seeded rows (from the design review, 2026-09-18)

| ID    | Documents involved                       | Conflict                                                  | Decision                                                                                     | Canonical owner                          | Verification                                         |
| ----- | ---------------------------------------- | --------------------------------------------------------- | -------------------------------------------------------------------------------------------- | ---------------------------------------- | ---------------------------------------------------- |
| C-001 | Development lessons / auto-expand test   | "Enable auto-expand" vs "no active consumer"              | Treat as nonfunctional until runtime-proved; auto-expand is configuration observability only | `aphrodite-auto-expand-testing`          | Fixture test (payload above threshold, before/after) |
| C-002 | Release workflow / release flow          | Tag-triggered registry publish behavior                   | Inspect actual workflow at tag commit; build a trigger table before any tag                  | `aphrodite-release-flow` (trigger audit) | Workflow trigger table at the exact commit           |
| C-003 | Hook reference / historical cache sample | Rust BLAKE3 live implementation vs Python SHA-256 history | Historical sample moves to archive; live docs carry only the Rust implementation             | `aphrodite-ccr-protocol`                 | Source-derived cache test                            |
| C-004 | Old branch flow / release flow           | Competing release ceremony                                | Old flow is archival only; the current release flow is the sole ceremony                     | `aphrodite-release-flow`                 | Skill manifest status (SKILL-MANIFEST.md)            |

## Rows from the 2026-09-20 rewrite wave (source-verified)

| ID    | Documents involved                                             | Conflict                                                                                                                                                                                            | Decision                                                                                                           | Canonical owner                           | Verification                                 |
| ----- | -------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ | ----------------------------------------- | -------------------------------------------- |
| C-005 | package.json / Cargo.toml                                      | package.json `1.4.6` vs crates `1.5.0`                                                                                                                                                              | Version ledger is the authority; bump package.json in the 1.5.0 ceremony                                           | `aphrodite-release-workflow`              | Version ledger probe                         |
| C-006 | AGENTS.md / verified manifests                                 | AGENTS.md claimed binary 1.4.6 / plugin 2.1.4 / release-notes v1.4.0…v1.4.3 and taught auto-expand as a dev aid                                                                                     | AGENTS.md corrected to 1.5.0 / 2.2.0 / v1.4.0…v1.4.6; auto-expand declared inert config                            | Parent maintenance (AGENTS.md is the map) | Source scan at each release                  |
| C-007 | hook-contracts / Rust comment lib.rs:470-472                   | Hermes passes `tool_call_id` to `transform_terminal_output` (terminal_tool_result.py:144); comment claims only command/output/returncode/task_id/env_type                                           | Comment is stale; the contract lists the verified kwargs and `**kwargs`                                            | `aphrodite-hook-contracts`                | invoke_hook source scan                      |
| C-008 | hook-contracts / old reference notes                           | `pre_llm_call` passes `parent_session_id`; absent from all old notes                                                                                                                                | Contract updated with the verified keyword set                                                                     | `aphrodite-hook-contracts`                | turn_context.py scan                         |
| C-009 | plugin.yaml / Hermes source                                    | `pre_api_request`/`post_api_request` now have invocation sites (turn_api_request.py:52-63, turn_response_intake.py:67-69) but the plugin registers neither                                          | Not a defect; documented as available-but-unregistered. Re-register only via a deliberate change                   | `aphrodite-hook-contracts`                | plugin.yaml + Hermes scan                    |
| C-010 | context-engine-contract / old notes                            | `update_model` has no trailing `**kw`; one-engine-per-manager enforced (plugins.py:705); `compression.enabled` defaults true (cli.py:405)                                                           | Contract states the verified ABC signature and registration constraints                                            | `aphrodite-context-engine-contract`       | ABC signature + registration source          |
| C-011 | health-check-pattern.md (stale) / engine-observability         | Doc claimed `/health` returns `healthy\|degraded` and probes upstream; verified `/health` ALWAYS 200 `{"status":"healthy",...}` and never calls upstream; upstream is TTL-cached `/health/upstream` | Runbook corrected; local health and upstream reachability are separate probes                                      | `aphrodite-engine-observability`          | curl both endpoints                          |
| C-012 | ccr-marker-format.md (stale) / ccr-protocol                    | SHA-256/12-16-hex claim vs verified BLAKE3 first-40 lowercase hex; `mode` field does not exist; preview lives outside the marker                                                                    | Reference corrected with a staleness banner; the skill states the verified grammar and the diff vs the proposed v1 | `aphrodite-ccr-protocol`                  | proxy_format_ccr_output + compute_key source |
| C-013 | recompression-guard.md (stale) / compression-safety            | Python `_transform_*` + `_CCR_RE` guards no longer exist (plugin is a pure loader; skip logic moved into the Rust dylib)                                                                            | Skip rule is canonical; the Python implementation note is marked historical                                        | `aphrodite-compression-safety`            | Plugin source scan                           |
| C-014 | ccr-infinite-recursion.md (stale names) / ccr-protocol         | `headroom_retrieve`/`headroom_stats` retired → `aphrodite_*` family; recursion rule enforced (depth 5 + visited set)                                                                                | Reference updated; depth 5 and visited-hash set are the verified guard                                             | `aphrodite-ccr-protocol`                  | resolve/recursive.rs source                  |
| C-015 | proxy dispatch / tool-testing                                  | Internal relay `aphrodite_list` vs public tool `aphrodite_catalog`                                                                                                                                  | Tracked as divergence risk; public name is canonical for agents                                                    | `aphrodite-tool-testing`                  | Proxy dispatch scan                          |
| C-016 | live tool catalog / plugin.yaml                                | `aphrodite_debug` is a 14th live-catalog tool but plugin.yaml registers 13                                                                                                                          | Documented as runtime-derived; inventory stays 13 with debug noted                                                 | `aphrodite-tool-testing`                  | Live catalog vs plugin.yaml                  |
| C-017 | CEREMONY.md / HANDOFF.md / RELEASE-METHODOLOGY.md vs workflows | Docs claimed Current's Auto.yml pushes `branch: Development` (leak); verified at `a81acab6` both copies push their own branch                                                                       | Leak is fixed; the B4 audit stays mandatory and the fix is recorded as evidence                                    | `aphrodite-release-flow`                  | Workflow scan at the release commit          |
| C-018 | tool-testing old doctrine                                      | "Preview IS the answer - skip retrieval" vs retrieve-first                                                                                                                                          | Retrieve-first is canonical; previews are decision aids, never substitutes                                         | `aphrodite-tool-testing`                  | Marker resolution round trip                 |

## Schema for new entries

| Column             | Required content                                                |
| ------------------ | --------------------------------------------------------------- |
| ID                 | `C-###`, sequential; never reused                               |
| Documents involved | Both/all documents that conflicted                              |
| Conflict           | The two claims in tension, stated precisely                     |
| Decision           | The binding rule, in one sentence                               |
| Canonical owner    | The single skill that owns the decision (see SKILL-MANIFEST.md) |
| Verification       | The probe or test that proves the decision holds                |

## Rules

- A rule that appears in only one skill and is not in tension is not a
  contradiction; do not log it.
- When two live skills state the same rule, that is a duplication violation
  (BOUNDARIES.md ownership precedence), not a contradiction - fix the copy,
  then optionally note the fix here.
- Every archived skill (ARCHIVE-INDEX.md) that conflicts with a live skill
  must be traceable to a row here.
- New rows added by concurrent rewrite agents: append, never renumber. If a
  seeded row's decision changes, append a new row whose "Documents involved"
  includes the old row's ID.
- The register itself is governance metadata; it is updated in the same
  change that resolves the contradiction.

## Open room for concurrent agents

Rows C-005 through C-018 were logged by the 2026-09-20 rewrite wave (see
table above). Any future conflict found during development appends C-019+
following the schema below.
