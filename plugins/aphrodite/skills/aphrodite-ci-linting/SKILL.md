---
name: aphrodite-ci-linting
description: "CI and local linting for aphrodite — ruff, pyright, pre-release verification. Plugin import chain testing."
version: 1.0.0
platforms: [macos, linux]
---

# Aphrodite CI Linting

## Local Checks (run before commit/release)

```bash
# 1. Ruff — zero errors
ruff check plugins/aphrodite/ scripts/ crates/

# 2. Pyright — zero errors (strict mode, config in plugins/aphrodite/pyrightconfig.json)
npx pyright plugins/aphrodite/

# 3. Full plugin import chain — all modules must load
python3 -c "
import sys; sys.path.insert(0, 'plugins')
import aphrodite
print('plugin v' + str(aphrodite.PLUGIN_VERSION) + ' OK')
"
```

## CI (Check.yml)

CI runs `ruff check plugins/aphrodite/` + `npx pyright plugins/aphrodite/` on every push/PR to `Current` branch.

## Common Fixes

### Ruff
- `I001` unsorted imports → `ruff check --fix`
- `E402` late imports (logging disable) → `# noqa: E402`
- `SIM105` try/except/pass → `with contextlib.suppress(Error):`
- `UP015` redundant `"r"` mode → remove

### Pyright
- Missing backport import → `# type: ignore[import-not-found]`
- Dict narrowing → add type annotation: `_state: dict[str, Any]`
- None guard → explicit `is None` check before calling
- Arg mismatch → verify function signature matches call site
