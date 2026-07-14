# Aphrodite Project Rules ✨

These aren't restrictions - they're recipes for velocity. Follow them and
you'll ship faster, break nothing, and feel great about every commit. 💪

1. **Test without interrupting**: Use WezTerm MCP pane 1 for testing.
   Your main session is sacred - protect it like a VIP.

2. **Commit fearlessly, push confidently**: Every feature gets its own commit.
   Push to `PlayForm/Aphrodite` immediately. Small, frequent commits = zero
   merge pain = pure development joy.

3. **Proxy acting up? Kill it with kindness**: `lsof -ti :9797 :9798 | xargs kill`
   when compression interferes. It takes 2 seconds and fixes everything.

4. **Feed the proxy its key**: Always `export APHRODITE_API_KEY=...` before
   `cargo watch`. The proxy can't work miracles without credentials.

5. **Green means go**: Ruff 0 errors. Pyright 0 errors. Before every commit.
   We earned that clean CI badge - let's keep it glowing. 🟢

6. **One watch to rule them all**: `cargo watch -x 'build -p aphrodite -p aphrodite-hermes' -x 'run -p aphrodite'` -
   single command, instant rebuilds, pure flow state. Build BOTH packages, not
   just `aphrodite`: `cargo build -p aphrodite` alone never touches
   `libaphrodite_hermes.dylib` (it's a sibling package, not a dependency of
   `aphrodite`), so editing shared code and watching only the proxy pane gives
   zero signal about whether the Hermes plugin's dylib changed at all.

7. **Scripts have a home**: `Maintain/scripts/` for build, release, bench, ops.
   Root directory stays clean and beautiful.

Remember: every fix you make here saves real tokens, real money, and real time
for real people. You're not just writing code - you're building the future of
efficient AI. 🌟
