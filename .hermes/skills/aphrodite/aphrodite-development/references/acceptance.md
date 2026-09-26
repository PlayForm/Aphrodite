# Acceptance definitions (source vs installed)

Worked criteria consumed by the Acceptance definitions section of
`aphrodite-development` (SKILL.md). A change is done only when its mode's
acceptance list passes with recorded output - report what commands printed,
never "should pass" (AGENTS.md quality gates).

## Source-mode acceptance

- Workspace discovered.
- The intended crate builds from source.
- Plugin points at the repository source.
- Restart/reload has occurred.
- Updated binary/version is observed.
- Targeted contract test passes.

## Installed-mode acceptance

- No workspace is required.
- Plugin layout validates.
- Compatible release artifact resolves.
- Download/checksum/version pairing succeeds.
- Plugin starts against the installed binary.
- Targeted contract test passes.
