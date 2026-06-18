# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| ≥ 0.5.x | ✅ Stable releases |
| < 0.5.0 | ❌ Pre-release     |

---

## Reporting a Vulnerability

**Do not open public GitHub issues for security vulnerabilities.**

Send reports to **security@playform.cloud** with:

- A description of the vulnerability
- Steps to reproduce (proof of concept preferred)
- Affected versions (binary version, plugin version, or commit hash)
- Any mitigating factors

You should receive an acknowledgement within **48 hours**. We aim to triage
within **5 business days** and release a fix within **14 days** of confirmation
for critical/high-severity issues.

### What to expect

| Phase           | Timeline         | Details                                                                                      |
| --------------- | ---------------- | -------------------------------------------------------------------------------------------- |
| Acknowledgement | ≤48 h            | Confirmation of receipt                                                                      |
| Triage          | ≤5 business days | Severity assessment, impact scope                                                            |
| Fix             | ≤14 days         | Patch release for critical/high. Lower-severity issues may queue for the next release cycle. |
| Disclosure      | After fix        | Public advisory via GitHub Security Advisories                                               |

If the vulnerability is time-sensitive (0-day in the wild), we'll expedite to
**≤72 hours**.

---

## Threat Model

Aphrodite is a local compression proxy for Hermes Agent. It runs on the user's
machine, not on a remote server.

### Threats

| Threat                      | Impact | Mitigation                                                                      |
| --------------------------- | ------ | ------------------------------------------------------------------------------- |
| Prompt data exfiltration    | High   | No logging of prompt content to disk (cache mode). SQLite store is local-only.  |
| API key theft               | High   | API keys read from environment only. Never logged. Never persisted.             |
| CCR cache tampering         | Medium | SQLite WAL mode with file permissions; no network-accessible SQL interface.     |
| DoS via large payloads      | Medium | Token-budget limits enforced before compression. Configurable.                  |
| SSRF via provider redirects | Medium | URL validation on proxy routes; no follow-redirects to internal IPs by default. |

### Trust Boundaries

```
┌──────────────────┐     ┌──────────────────┐     ┌──────────────────┐
│   Hermes Agent   │────▶│  Aphrodite Proxy │────▶│   LLM Provider   │
└──────────────────┘     └────────┬─────────┘     └──────────────────┘
                                  │
                         ┌────────┴─────────┐
                         │                   │
                   ┌─────▼─────┐     ┌──────▼──────┐
                   │ Cache CCR │     │  Token CCR  │
                   │ (memory)  │     │  (SQLite)   │
                   └───────────┘     └─────────────┘
```

- **Hermes Agent → Aphrodite**: Loopback only (127.0.0.1). No authentication by
  default.
- **Aphrodite → LLM Provider**: TLS with rustls. No system OpenSSL dependency.
- **CCR stores**: In-memory (cache mode, :9797) or local SQLite (token mode,
  :9798). No remote access.

---

## Security Controls

### Authentication

| Surface         | Auth Method                                              |
| --------------- | -------------------------------------------------------- |
| Proxy endpoints | API key via `X-API-Key` header (optional, default: none) |
| Health          | None                                                     |
| Metrics         | None                                                     |

### Authorization

- **Single-tenant** by design - no multi-user support
- **API keys** for proxy authentication when enabled
- **CORS** configurable via `aphrodite.toml`

---

## Data Protection

| Data         | At Rest                                                         | In Transit      |
| ------------ | --------------------------------------------------------------- | --------------- |
| Prompts      | Encrypted (if SQLite encrypted via filesystem-level encryption) | TLS to provider |
| Responses    | Encrypted (same)                                                | TLS to provider |
| API keys     | Environment only (never persisted)                              | TLS to provider |
| CCR payloads | Plain text in SQLite WAL (token mode)                           | Loopback only   |
| Metrics      | Plain text                                                      | Loopback only   |

### CCR Store Privacy

In **cache mode** (:9797), CCR entries live in memory only - process restart
purges everything. In **token mode** (:9798), CCR entries persist in
`~/Library/Application Support/aphrodite/ccr.db` (SQLite WAL). Consider enabling
full-disk encryption or an encrypted APFS volume for sensitive environments.

---

## Input Validation

- **Payload size**: Enforced via configurable token budget (default: 100 KB
  threshold)
- **URL validation**: Provider URLs validated before forwarding
- **Schema validation**: All API inputs validated via Serde deserialization

---

## Secrets Management

Aphrodite reads API keys exclusively from environment variables:

```bash
# Provider key (required)
export APHRODITE_API_KEY=sk-...

# Upstream API key (optional, forwarded as X-API-Key)
export APHRODITE_UPSTREAM_API_KEY=sk-...
```

- **Never** written to disk
- **Never** logged at any log level
- **Never** included in CCR markers or database entries

---

## Supply Chain Security

### Dependencies

- **Workspace dependencies** in `Cargo.toml` - single source of truth for
  version pinning
- **Pinned transitive deps** via `Cargo.lock` - committed to the repo
- **Audit**: `cargo audit` run regularly (CI gate)
- **cargo-deny**: `deny.toml` at project root for license + advisory + duplicate
  checks
- **No npm/PyPI deps** - pure Rust dependency tree (except headroom-core
  vendored source)

### Build

- **Reproducible builds** - same commit produces the same binary on the same
  target
- **Static linking** - single binary with no runtime system deps (rustls
  everywhere, no openssl-sys)
- **No vendored C libraries** - all dependencies are pure Rust or bundle their
  runtime (SQLite via `bundled` feature, ONNX Runtime via `ort`)

### Verifying releases

Release binaries are built via GitHub Actions. Verify the tag matches the commit
in `Cargo.toml`:

```bash
git verify-tag v0.5.68
cargo build --release -p aphrodite --locked
```

---

## Vulnerability Disclosure Policy

We follow **Coordinated Disclosure**:

1. Reporter submits details to **security@playform.dev**
2. We triage and confirm the issue
3. We develop and test a fix
4. We release the fix and publish an advisory
5. Reporter may disclose publicly after the advisory is published

We will credit reporters in the advisory unless they request anonymity.

---

## Version History

| Version | Date       | Changes                                 |
| ------- | ---------- | --------------------------------------- |
| 1.0.0   | 2026-06-16 | Initial security document for Aphrodite |
