# Aphrodite Documentation

Aphrodite compresses context before it hits the LLM - through a reverse proxy
for any OpenAI-compatible client, or as a native Hermes plugin with hook-level
interception. CCR (Compress-Cache-Retrieve) storage, a 30-type classifier,
context engine, and prefetch pipeline - all under 1ms. This tree documents
the 1.5.0 binary and 2.2.0 plugin.

## Getting Started

- [Installing Aphrodite](install/README.md) - which artifact you need (proxy
  binary vs. Hermes plugin), with a decision tree
- [Windows Install](install/windows.md) - fast path with `download.ps1`
  (native PowerShell, no `bash` needed)
- [macOS/Linux Install](install/macos-linux.md) - `download.sh`,
  `aphrodite setup`, building from source
- [Troubleshooting](install/troubleshooting.md) - proxy not auto-launching,
  verifying the proxy without a full Hermes session, the two-config-files trap

## Architecture

- [Architecture Index](architecture/README.md) - all 11 flow traces: startup,
  chat compression, retrieve, hook/FFI, CCR lifecycle, SSE streaming, config
  resolution, dylib hot-reload, release CI, components, data model

## CCR (Compress-Cache-Retrieve)

- [Marker Format](ccr/marker-format.md) - `<<<CCR:hash|type|size>>>` schema
  with BLAKE3 hash, 30 content types, TOML-driven preview templates
- [Lifecycle](ccr/lifecycle.md) - the 6-phase flow: compress, retrieve,
  expire (TTL+LRU+debounce), with threshold tables per type and mode
- **Backends**
    - [SQLite](ccr/backends/sqlite.md) - schema, WAL mode, lazy TTL purge
    - [In-Memory](ccr/backends/in-memory.md) - DashMap + VecDeque, capacity
      10,000, TOCTOU-safe eviction
    - [Inline](ccr/backends/inline.md) - LRU cache, 1024 entries, lock-safety
      pattern

## Classification

- [Content Types](classification/content-types.md) - the 30-type taxonomy with
  detection order, threshold groups, and preview forms
- [Classification Index](classification/README.md)

## Config & Install

- [aphrodite.toml](config/aphrodite-toml.md) - full schema: `[[proxies]]`,
  `[defaults]`, `[compression]`, `[previews]`, `[templates.*]`, `[flow]`,
  precedence rules, hot-reload
- [Environment Variables](config/env-vars.md) - the `APHRODITE_*` registry
  with the documented-but-unwired list

## Proxy

- [Architecture](proxy/architecture.md) - two-listener model (:9797 cache +
  :9798 token), routing table, management-route bearer auth, SSE pass-through
- [Handlers](proxy/handlers.md) - all 8 handlers with their behaviors
- [Retry](proxy/retry.md) - connect-phase retry scope, backoff, counters
- [Compression](proxy/compression.md) - detect → threshold → hash → store →
  marker pipeline, budget curve, smart markers

## API & Metrics

- [Health](api/health.md) - `GET /health` liveness, `X-Aphrodite-Fill-Pct`
- [Metrics Endpoint](api/metrics-endpoint.md) - `GET /metrics` Prometheus text
- [Retrieve](api/retrieve.md) - `POST /retrieve` hash lookup, query, pagination
- [CCR Endpoints](api/ccr-endpoints.md) - create/list/delete/reload with the
  error table
- [Prometheus](metrics/prometheus.md) - all 28 metric names, types, labels
- [Queries](metrics/queries.md) - PromQL reference: CCR hit rate, latency

## Plugin

- [Hooks](plugin/hooks.md) - the SIX hooks with purpose and fire-time table
- [Directives](plugin/directives.md) - built-in set, injection order,
  materialization
- [Context Engine](plugin/context-engine.md) - the opt-in pass-through engine

## Tool Relay

- [Tools](tool-relay/tools.md) - the 13 tools with schemas and behavior
- [Callbacks](tool-relay/callbacks.md) - transform-hook surface + HTTP relay

## Examples & Guides

- [CCR Examples: What the LLM Sees](examples/llm-view.md) - real captured
  markers, preview families, token economics
- [Hermes Integration](guides/hermes-integration.md) - pure-loader plugin,
  runtime home, self-healing layout
- [Hermes Tool Output Schemas](guides/hermes-tool-output-schemas.md) - the
  43-shape tool-by-tool output catalog

## Release Notes

- [Release Notes Index](release-notes/README.md) - v1.4.0 through v1.4.6,
  what shipped in each

## Roadmap

- [Centers](centers.md) - AI-conversation memory annotations; shipped vs.
  sketch items labeled

## Style Guide

Every doc in this tree follows one style, the same one this page and the
root `README.md` use:

| Rule                      | What it means                                                                                                                                 |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| Explain, then detail      | Open with one or two plain sentences on what the thing is and why it exists, then drop into tables/code                                       |
| Tables over prose         | Fields, flags, options, comparisons - anything with more than two rows of structured data - are a table, not a bulleted wall of text          |
| No file/line citations    | Docs describe behavior directly; they don't cite exact source files or line numbers as proof - accuracy is a writing standard, not a footnote |
| No placeholder content    | If a documented setting or feature isn't confirmed to do anything, the doc says so plainly instead of presenting it as working                |
| Minimal external links    | Link out only when the reader needs to click through to do something (download a release, read an upstream project's own docs)                |
| Roadmap ideas are labeled | Forward-looking or unimplemented designs (see [Centers](centers.md)) say clearly which parts are shipped and which are sketches               |
