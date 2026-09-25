# Petdex - Animated Pet Mascots

Use when the user wants an animated "pet" mascot in the Hermes CLI, TUI, or
desktop app - including inside an Aphrodite dev session (`hermes --profile
dev-aphrodite`). Browse, install, and select pets from the public
[petdex](https://github.com/crafter-station/petdex) gallery. An installed pet
reacts to agent activity (idle, running a tool, reviewing, error, done) across
every surface. This reference drives the `hermes pets` CLI and the
`display.pet` config - it does not generate sprites.

## When to Use

- The user wants a desktop/terminal mascot or asks about "pets" / petdex.
- The user wants to change, preview, or disable the active pet.
- Diagnosing why a pet isn't showing (terminal graphics support, config).

## Prerequisites

- Network access to `petdex.dev` for the gallery/manifest (read-only, no auth).
- Pillow (a core Hermes dependency) for sprite decoding - already installed.
- For full-fidelity terminal rendering: a graphics-capable terminal (kitty,
  Ghostty, WezTerm, iTerm2, or sixel). Otherwise a truecolor Unicode
  half-block fallback is used automatically.

## How to Run

Use the `terminal` tool to run `hermes pets <subcommand>`.

## Quick Reference

| Goal                        | Command                                                                |
| --------------------------- | ---------------------------------------------------------------------- |
| Browse the gallery          | `hermes pets list` (add a substring to filter: `hermes pets list cat`) |
| List installed pets         | `hermes pets list --installed`                                         |
| Install a pet               | `hermes pets install <slug>` (add `--select` to make it active)        |
| Set the active pet          | `hermes pets select <slug>` (omit slug for a picker)                   |
| Resize the pet everywhere   | `hermes pets scale <factor>` (e.g. `0.5`, clamped 0.1-3.0)             |
| Preview/animate in terminal | `hermes pets show [slug] [--cycle] [--state run]`                      |
| Disable the pet             | `hermes pets off`                                                      |
| Remove a pet                | `hermes pets remove <slug>`                                            |
| Diagnose setup              | `hermes pets doctor`                                                   |

## Procedure

1. Find a pet: `hermes pets list <query>` and note its `slug`.
2. Install + activate: `hermes pets install <slug> --select`.
3. Preview it: `hermes pets show` (Ctrl+C to stop).
4. Confirm setup: `hermes pets doctor` - shows the resolved pet, configured
   render mode, detected terminal graphics protocol, and effective mode.

Pets install into `<HERMES_HOME>/pets/<slug>/` (profile-aware: under the
`dev-aphrodite` profile that is `~/.hermes/profiles/dev-aphrodite/pets/`).
Selecting a pet writes `display.pet.slug` + `display.pet.enabled` to
`config.yaml`.

## Configuration

Under `display.pet` in `config.yaml`:

- `enabled` (bool) - master on/off.
- `slug` (str) - active pet; empty = first installed.
- `render_mode` - `auto` (detect) | `kitty` | `iterm` | `sixel` | `unicode` | `off`.
- `scale` (float) - on-screen size of the native 192×208 frames (default 0.33,
  clamped 0.1-3.0). One knob resizes every surface; set it with
  `hermes pets scale <factor>`, the `/pet scale` slash command, or the desktop
  Appearance slider.
- `unicode_cols` (int) - width in columns for the Unicode fallback.

## Pitfalls

- A pet only shows once one is installed AND selected (`enabled: true`).
- Inside a pipe/redirect (no TTY) terminal rendering is disabled by design.
- The petdex gallery also ships a standalone CLI that installs to its own
  location; Hermes uses its own profile-scoped `<HERMES_HOME>/pets/` instead -
  install through `hermes pets`, never a third-party CLI.

**Stop if** a pet "doesn't work" and `hermes pets doctor` reports a terminal
graphics problem - the pet is fine; the terminal is the issue.

**Recovery** - permitted: fall back to `render_mode: unicode` or switch to a
graphics-capable terminal. Prohibited: hand-editing `config.yaml` to force
`enabled: true` (use `hermes pets select`).

## Verification

| Claim                       | Test                                    | Pass condition                           |
| --------------------------- | --------------------------------------- | ---------------------------------------- |
| Pet installs profile-scoped | `hermes pets install <slug> --select`   | Files under `<HERMES_HOME>/pets/<slug>/` |
| Setup is healthy            | `hermes pets doctor`                    | Reports `✓ ready`                        |
| Toggle lands in config      | `hermes config get display.pet.enabled` | `true` after select                      |

`hermes pets doctor` reports `✓ ready` when a pet is installed, selected,
enabled, and Pillow is importable.
