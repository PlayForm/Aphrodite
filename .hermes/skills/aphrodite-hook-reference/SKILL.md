---
name: aphrodite-hook-reference
description: "Use when deciding which aphrodite hook/context contract to consult. Dispatcher: routes each hook, context-engine, CCR, safety, and observability subject to its canonical owner skill."
version: 2.0.0
platforms: [macos]
tags: [aphrodite, hermes, hooks, dispatcher, index]
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
    - The subject-to-skill routing table for hook/context/CCR/safety/observability questions
    - The retrieve-first doctrine pointer (doctrine itself is owned by aphrodite-tool-testing)
    - The evidence-references routing map (where the relocated hook/context-engine evidence files now live)
depends_on:
    - aphrodite-boundaries
    - aphrodite-orientation
    - aphrodite-hook-contracts (hook names, signatures, return semantics)
    - aphrodite-context-engine-contract (ContextEngine registration/selection)
    - aphrodite-ccr-protocol (marker grammar, parser, resolution)
    - aphrodite-compression-safety (pairing, skip lists, recursion caps)
    - aphrodite-engine-observability (health layers, cache contract, probes)
supersedes:
    - aphrodite-hook-reference v1.x (monolithic contract; content split into the five contract skills)
verification:
    source_of_truth:
        - .hermes/governance/SKILL-MANIFEST.md (routing ownership rows)
        - The canonical skills named in the table below (each owns its subject)
mutation_level: read-only
---

# Aphrodite Hook Reference - Dispatcher

This skill used to be the monolithic API reference for Hermes hook
invocations, ContextEngine registration, CCR marker format, compression
safety, and health-check design. That content has been split: each subject
now has exactly one canonical owner. **This file is a short dispatcher** -
consult it only to decide which skill to read, then go there. Do not copy
rules back into this file; the owner skills are the single source.

## Subject → canonical skill

| Subject                                                                                                                                                             | Canonical skill                                                                                                                                    |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hook registration names, invocation sites, keyword parameters, return semantics, nullability, known mismatches (`output` vs `stdout`, `on_` prefix, copy semantics) | `aphrodite-hook-contracts`                                                                                                                         |
| ContextEngine ABC, isinstance registration gate, engine selection, configuration ownership, Hermes-vs-Aphrodite compression boundary                                | `aphrodite-context-engine-contract`                                                                                                                |
| CCR marker grammar (`<<<CCR:hash\|type\|size>>>`), typed parser contract, versioning, idempotence                                                                   | `aphrodite-ccr-protocol`                                                                                                                           |
| Tool-call/tool-result pairing, retrieval skip list, recursion depth, re-compression guard, failure-behavior policy                                                  | `aphrodite-compression-safety`                                                                                                                     |
| Health layers, `/health` vs `/health/upstream`, cache contract, log locations, telemetry                                                                            | `aphrodite-engine-observability`                                                                                                                   |
| Retrieve-first doctrine (when a `<<<CCR:...>>>` marker appears, resolve it via `aphrodite_retrieve` before any other action)                                        | `aphrodite-tool-testing`                                                                                                                           |
| Historical evidence: hook invocation verification method, parameter-mismatch incidents, ContextEngine API/integration/pitfalls, session discoveries                 | `aphrodite-hook-contracts/references/` + `aphrodite-context-engine-contract/references/` (evidence only; the owner skills define current behavior) |

## Evidence references (relocated)

The historical evidence files no longer live in this skill's `references/`
directory - they were relocated to the canonical owners during the 2026-09-18
refactor (verified 2026-09-20):

- `hook-invocations.md`, `hook-invocation-verification.md`,
  `hook-parameter-mismatches.md` → `aphrodite-hook-contracts/references/`
- `context-engine-api.md`, `context-engine-integration.md`,
  `context-engine-pitfalls.md`, `session-discoveries-20260615.md` →
  `aphrodite-context-engine-contract/references/`

The references are evidence only; the owner skills define current behavior.

## Usage rule

- **Implementation/debugging a hook signature** → `aphrodite-hook-contracts`.
- **Working with the context engine** → `aphrodite-context-engine-contract`.
- **Producing/parsing/resolving markers** → `aphrodite-ccr-protocol`.
- **Deciding what may be compressed** → `aphrodite-compression-safety`.
- **Health, cache, telemetry** → `aphrodite-engine-observability`.
- **Operating a compressed session** → `aphrodite-operations`.

## Local test matrix

| Claim                                        | Evidence source                | Test                                                                   | Pass condition                               | Failure response                      |
| -------------------------------------------- | ------------------------------ | ---------------------------------------------------------------------- | -------------------------------------------- | ------------------------------------- |
| Dispatcher routes every subject to one owner | This table + SKILL-MANIFEST.md | For each row, open the canonical skill and confirm it owns the subject | One owner per subject; no subject owned here | Move the missing content to its owner |
| Dispatcher contains no operational rules     | This file                      | Grep for hook signatures, marker formats, or thresholds in this file   | Zero operational rules present               | Remove them; link to owner            |
| Retrieve-first doctrine is only pointed to   | `aphrodite-tool-testing`       | Load the tool-testing skill; confirm it states the doctrine            | Doctrine lives there, pointer here           | Fix the pointer                       |
| References stay evidence-only                | This file's references/        | Check each reference's subject against the routing table               | Each reference maps to a canonical owner     | Update routing table                  |
