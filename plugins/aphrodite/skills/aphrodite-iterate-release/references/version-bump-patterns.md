# Version Bump Patterns for aphrodite Iterate-Release

Exact `old_string`/`new_string` pairs for `patch` calls. Bump all 6 locations in lockstep.
Replace `X.Y.Z` with current version, `X.Y.Z+1` with next.

## 1. BIN_VERSION in __init__.py

```
old: BIN_VERSION = "vX.Y.Z"          # binary download version (must match Cargo.toml)
new: BIN_VERSION = "vX.Y.Z+1"          # binary download version (must match Cargo.toml)
```

## 2. PLUGIN_VERSION in __init__.py

```
old: PLUGIN_VERSION = "A.B.0"        # plugin version
new: PLUGIN_VERSION = "A.B+1.0"        # plugin version
```

## 3. Docstring in __init__.py

```
old: aphrodite vA.B.0 - Auto-install + launch aphrodite proxies.
new: aphrodite vA.B+1.0 - Auto-install + launch aphrodite proxies.
```

## 4. Cargo.toml version

```
old: version = "X.Y.Z"
new: version = "X.Y.Z+1"
```

## 5. plugin.yaml version

```
old: version: A.B.0
new: version: A.B+1.0
```

## 6. plugin.yaml install_message

```
old:   aphrodite vA.B.0 - <previous description>.
new:   aphrodite vA.B+1.0 - <new description>.
```

Also update `plugin.yaml` `description:` field to match the change summary.

## Full patch sequence (example from v0.5.7 → v0.5.8)

All 6 calls in one turn:

```python
patch(path="plugins/aphrodite/__init__.py",
      old_string='BIN_VERSION = "v0.5.7"',
      new_string='BIN_VERSION = "v0.5.8"')
patch(path="plugins/aphrodite/__init__.py",
      old_string='PLUGIN_VERSION = "1.16.0"',
      new_string='PLUGIN_VERSION = "1.17.0"')
patch(path="crates/aphrodite/Cargo.toml",
      old_string='version = "0.5.7"',
      new_string='version = "0.5.8"')
patch(path="plugins/aphrodite/plugin.yaml",
      old_string='version: 1.16.0',
      new_string='version: 1.17.0')
patch(path="plugins/aphrodite/__init__.py",
      old_string='aphrodite v1.16.0 - Auto-install + launch aphrodite proxies.',
      new_string='aphrodite v1.17.0 - Auto-install + launch aphrodite proxies.')
patch(path="plugins/aphrodite/plugin.yaml",
      old_string='  aphrodite v1.16.0 - <old>.',
      new_string='  aphrodite v1.17.0 - <new>.')
```
