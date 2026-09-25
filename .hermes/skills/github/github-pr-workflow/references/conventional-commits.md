# Conventional Commits Quick Reference

Format: `type(scope): description`

## Types

| Type       | When to use                            | Example                                                   |
| ---------- | -------------------------------------- | --------------------------------------------------------- |
| `feat`     | New feature or capability              | `feat(proxy): add health-check endpoint`                  |
| `fix`      | Bug fix                                | `fix(ccr): handle truncated marker hashes`                |
| `refactor` | Code restructuring, no behavior change | `refactor(plugin): extract loader into separate module`   |
| `docs`     | Documentation only                     | `docs: update API usage examples in README`               |
| `test`     | Adding or updating tests               | `test(ccr): add round-trip tests for marker expansion`    |
| `ci`       | CI/CD configuration                    | `ci: add Rust stable to the Build.yml test matrix`        |
| `chore`    | Maintenance, dependencies, tooling     | `chore: bump vendored deps in vendor/headroom`            |
| `perf`     | Performance improvement                | `perf(proxy): cache decoded payloads by hash`             |
| `style`    | Formatting, whitespace, semicolons     | `style: run cargo fmt on crates/aphrodite`                |
| `build`    | Build system or external deps          | `build: update the Cargo workspace edition`               |
| `revert`   | Reverts a previous commit              | `revert: revert "feat(proxy): add health-check endpoint"` |

## Scope (optional)

Short identifier for the area of the monorepo: `proxy`, `ccr`, `plugin`,
`hermes`, `cli`, etc. (`crates/aphrodite` = proxy/ccr engine,
`crates/aphrodite-hermes` = hermes bridge, `plugins/aphrodite` = plugin).

## Breaking Changes

Add `!` after type or `BREAKING CHANGE:` in footer:

```
feat(plugin)!: change the plugin entry-point signature

BREAKING CHANGE: plugins must now export the new init function.
Migration guide: https://docs.example.com/migrate-plugin
```

## Multi-line Body

Wrap at 72 characters. Use bullet points for multiple changes:

```
feat(proxy): add health-check endpoint

- Add /api/health route to the proxy
- Surface engine and dylib versions in the response
- Add integration tests for the endpoint

Closes #42
```

## Linking Issues

In the commit body or footer:

```
Closes #42          <- closes the issue when merged
Fixes #42           <- same effect
Refs #42            <- references without closing
Co-authored-by: Name <email>
```

## Quick Decision Guide

- Added something new? -> `feat`
- Something was broken and you fixed it? -> `fix`
- Changed how code is organized but not what it does? -> `refactor`
- Only touched tests? -> `test`
- Only touched docs? -> `docs`
- Updated CI/CD pipelines? -> `ci`
- Updated dependencies or tooling? -> `chore`
- Made something faster? -> `perf`
