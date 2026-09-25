# Surfaces (quick orientation)

Use when describing or operating one of Hermes's four surfaces - the desktop
app, the web dashboard, the Ink TUI, or the OpenAI-compatible proxy. Each
surface is a claim until `hermes --help` lists its subcommand.

- **Desktop app** (`hermes desktop` / `hermes gui`) - native Electron app for macOS/Linux/Windows: streaming chat, session list, Cmd+K palette, drag-and-drop files, native notifications, per-profile remote-gateway login. Extend it with UI plugins - `references/desktop-plugins.md`.
- **Web dashboard** (`hermes dashboard`) - full admin panel: messaging channels, MCP catalog, webhooks, memory, profile builder, plus an embedded `hermes --tui` chat. Secured behind an OAuth/token gate.
- **Ink TUI** (`hermes --tui` or `display.interface: tui`) - terminal UI with docked widget apps - `references/tui-widgets.md`.
- **OpenAI-compatible proxy** (`hermes proxy`) - a local OpenAI API backed by whichever OAuth provider you're signed into. It is not the Aphrodite CCR proxy (`crates/aphrodite`): that one compresses session context instead of serving an API.

The surface descriptions above are CLAIM: `hermes --help` lists each
subcommand, and the linked reference documents the rest.

## Local claim-to-test matrix

| Claim                                  | Evidence source | Test                     | Pass condition                  | Failure response               |
| -------------------------------------- | --------------- | ------------------------ | ------------------------------- | ------------------------------ |
| Each surface subcommand exists         | `hermes --help` | Run `hermes --help`      | `desktop`/`gui`/`dashboard`/`--tui`/`proxy` listed | Report the missing subcommand |
| CCR proxy is not the OpenAI proxy      | `hermes proxy`  | Compare `crates/aphrodite` behavior | `hermes proxy` serves an API; CCR proxy compresses context | Fix the conflation            |