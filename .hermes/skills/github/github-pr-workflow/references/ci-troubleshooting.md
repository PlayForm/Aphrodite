# CI Troubleshooting Quick Reference

Common CI failure patterns and how to diagnose them from the logs. Applies
to the Aphrodite repos' `Build.yml` (and `Publish.yml` release checks).
Scratch artifacts (downloaded logs, zips) belong under `~/.hermes/tmp/`,
never in the repo tree.

## Reading CI Logs

```bash
# With gh
gh run view < RUN_ID > --log-failed

# With curl - download and extract
mkdir -p ~/.hermes/tmp/ci-logs
curl -sL -H "Authorization: token ***" \
	https://api.github.com/repos/$GH_OWNER/$GH_REPO/actions/runs/ \
	-o ~/.hermes/tmp/ci-logs.zip < RUN_ID > /logs && unzip -o ~/.hermes/tmp/ci-logs.zip -d ~/.hermes/tmp/ci-logs
```

## Common Failure Patterns

### Test Failures

**Signatures in logs:**

```
test crates::aphrodite::proxy::tests::test_health ... FAILED
error[E0308]: mismatched types
---- proxy::tests::health_roundtrip stdout ----
assertion `left == right` failed: left: 42, right: 43
```

**Diagnosis:**

1. Find the test file and line number from the failure output
2. Use `read_file` to read the failing test
3. Check if it's a logic error in the code or a stale test assertion
4. Look for compile errors (`error[E...]`) - usually a type or borrow
   mismatch in the changed crate

**Common fixes:**

- Update assertion to match new expected behavior
- Fix the type/borrow error in the crate source
- Fix flaky test (add retry, mock external service, fix race condition)

---

### Lint / Formatting Failures

**Signatures in logs:**

```
crates/aphrodite/src/proxy.rs:45:1: warning: unused variable
error: diff in crates/aphrodite/src/ccr.rs (run `cargo fmt` to fix)
```

**Diagnosis:**

1. Read the specific file:line numbers mentioned
2. Check which gate is complaining (`cargo fmt --check`, `cargo clippy`,
   rustfmt)

**Common fixes:**

- Run the formatter locally: `cargo fmt --all`
- Run clippy and fix warnings: `cargo clippy --all-targets -- -D warnings`
- Fix the specific style violation by editing the file
- If using `patch`, make sure to match existing indentation style

---

### Type / Borrow Check Failures (clippy / rustc)

**Signatures in logs:**

```
error[E0308]: mismatched types
  --> crates/aphrodite/src/engine.rs:23:5
error[E0382]: borrow of moved value
```

**Diagnosis:**

1. Read the file at the mentioned line
2. Check the function signature and what's being passed

**Common fixes:**

- Add type cast or conversion
- Fix the function signature
- Adjust the borrow/lifetime (or `clone()` where ownership requires it)

---

### Build / Compilation Failures

**Signatures in logs:**

```
error: the lock file ... needs to be updated but `--locked` was passed
error: failed to run custom build command for `aphrodite v0.x.x`
The following warnings were emitted during compilation:
```

**Diagnosis:**

1. Check `Cargo.toml` / `Cargo.lock` for the missing or incompatible
   dependency
2. Compare local vs CI Rust toolchain version (`rust-toolchain` file vs the
   pinned version in `Build.yml`)

**Common fixes:**

- Add missing dependency to the workspace manifest
- Pin compatible version
- Update the lockfile (`cargo update` / `cargo build --locked` comparison)
- Vendored deps under `vendor/headroom` must match the manifest pins

---

### Permission / Auth Failures

**Signatures in logs:**

```
fatal: could not read Username for 'https://github.com': No such device or address
Error: Resource not accessible by integration
403 Forbidden
```

**Diagnosis:**

1. Check if the workflow needs special permissions (token scopes)
2. Check if secrets are configured (missing `GITHUB_TOKEN` or custom secrets
   such as the API key for the proxy tests)

**Common fixes:**

- Add `permissions:` block to workflow YAML
- Verify secrets exist: `gh secret list` or check repo settings
- For fork PRs: some secrets aren't available by design

---

### Timeout Failures

**Signatures in logs:**

```
Error: The operation was canceled.
The job running on runner ... has exceeded the maximum execution time
```

**Diagnosis:**

1. Check which step timed out
2. Look for infinite loops, hung processes, or slow network calls (e.g. an
   engine binary that never exits)

**Common fixes:**

- Add timeout to the specific step: `timeout-minutes: 10`
- Fix the underlying performance issue
- Split into parallel jobs

---

### Docker / Container Failures

**Signatures in logs:**

```
docker: Error response from daemon
failed to solve: ... not found
COPY failed: file not found in build context
```

**Diagnosis:**

1. Check the Dockerfile for the failing step
2. Verify the referenced files exist in the repo

**Common fixes:**

- Fix path in COPY/ADD command
- Update base image tag
- Add missing file to `.dockerignore` exclusion or remove from it

---

## Auto-Fix Decision Tree

```
CI Failed
├── Test failure
│   ├── Assertion mismatch -> update test or fix logic
│   └── Compile error -> fix types/borrows in the crate
├── Lint failure -> run cargo fmt / clippy, fix style
├── Type error -> fix types
├── Build failure
│   ├── Missing dep -> add to workspace manifest
│   └── Version conflict -> update pins / lockfile
├── Permission error -> update workflow permissions (needs user)
└── Timeout -> investigate perf (may need user input)
```

## Re-running After Fix

```bash
git add <fixed_files> && git commit -m "fix: resolve CI failure" && git push

# Then monitor
gh pr checks --watch 2>/dev/null || \
  echo "Poll with: curl -s -H 'Authorization: token ***' https://api.github.com/repos/.../commits/$(git rev-parse HEAD)/status"
```
