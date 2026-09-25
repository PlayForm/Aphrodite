# Verification checklist

Run before closing any verification session:

- [ ] Dylib loaded via the plugin's `_load_dylib()` / `_call_json()`, zero raw `ctypes.CDLL` in the probe
- [ ] Version handshake: dylib version == BINARY_VERSION (fresh process, not stale dylib)
- [ ] Release dylib rebuilt, not stale
- [ ] Real test suites run and ACTUAL numbers recorded
- [ ] No stray `APHRODITE_*` env vars exported (config tests hermetic: remove_var/restore, or monkeypatch.delenv)
- [ ] Scratch in `.hermes/tmp/`, never `/tmp`
- [ ] No crash dialogs, repro SURVIVED, zero new SIGSEGV
- [ ] Docs pass documentation linting (L-1..L-10) and prettier
