# Headroom fork publish - dispatch and post-event verification

The `vendor/headroom` fork crate (`aphrodite-headroom-core`) is
dispatch-gated, NOT tag-reachable: a tag push never publishes it. Publishing
it is a separate, deliberate event that requires the fork leg (Step I5) to
have run first.

## Dispatch conditions

Dispatch `gh workflow run Publish -f publish_crates=true` ONLY when Step I5
carried a fork delta (fork crate + parent pin bumped, fork tag + gitlink
float done). If the fork leg did not run, the version check sees the old
version live on crates.io and skips the stale version silently (the 1.5.0
published-version trap) - never dispatch for `aphrodite` /
`aphrodite-hermes`: the tag push already ran `cargo publish` for them per
the accepted Gate R7 trigger audit.

Needs chain (Publish.yml): Test → Publish-Headroom-Core → Publish-Aphrodite
→ Publish-Hermes - headroom publishes FIRST. `aphrodite-headroom-core` is
index-checked and skipped if already live.

## Post-event consumer verification

```sh
curl -A < ua > https://crates.io/api/v1/crates/aphrodite-headroom-core | grep max_version
```

The index must serve the NEW fork version. Seeing only the old 0.1.2 means
the skip fired again - the fork version was not published. Verify the fork
tag exists before dispatch and the parent gitlink floats to the TAGGED fork
commit (CI publishes the parent-recorded gitlink tree, not the local
submodule HEAD).
