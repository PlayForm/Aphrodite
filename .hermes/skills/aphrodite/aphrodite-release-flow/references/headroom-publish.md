# Headroom fork publish - chain behavior and post-event verification

The `vendor/headroom` fork crate (`aphrodite-headroom-core`) is never
published by the release chain: its publish step is gated on the REMOVED
`workflow_dispatch publish_crates` input, so the `workflow_run`-chained
`Publish.yml` never fires it. Publishing it is a separate, deliberate event
(restored gate or out-of-band `cargo publish` with `CARGO_REGISTRY_TOKEN`)
that requires the fork leg (Step I5) to have run first.

## When the fork crate can publish

There is NO dispatch input: `Publish.yml` runs via `workflow_run` on Build
completion, and its headroom-core publish step is unreachable (gated on the
removed input). Step I5's fork leg (fork crate + parent pin bumped, fork tag
+ gitlink float done BEFORE the parent tag push) must still run - a stale
fork version either ships silently (the 1.5.0 published-version trap) or
fails `Publish-Aphrodite` (its `path + version` dep must already exist on
crates.io). Never re-trigger for `aphrodite` / `aphrodite-hermes`: the tag →
Build → Publish chain already ran `cargo publish` for them per the accepted
Gate R7 trigger audit.

Needs chain (Publish.yml): Test → Publish-Headroom-Core → Publish-Aphrodite
→ Publish-Hermes. The headroom-core CHECK step still runs (index-checked),
but the PUBLISH step's `if:` names the removed `workflow_dispatch
publish_crates` input - it never fires under the `workflow_run`-only `on:`.

## Post-event consumer verification

```sh
curl -A < ua > https://crates.io/api/v1/crates/aphrodite-headroom-core | grep max_version
```

The index must serve the NEW fork version. Seeing only the old 0.1.2 means
the fork version was not published - under the current chain it CANNOT be
(the publish step is unreachable). Verify the fork tag exists before the
release chain and the parent gitlink floats to the TAGGED fork commit (CI
publishes the parent-recorded gitlink tree, not the local submodule HEAD).
