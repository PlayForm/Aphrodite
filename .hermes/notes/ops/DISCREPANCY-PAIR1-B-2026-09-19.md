# DISCREPANCY-PAIR1-B-2026-09-19

Verified every claim in .hermes/uml/07-11 against live source (2026-09-19,
branch Development). No substantive behavioral contradiction found; the only
drift is key-call-site line numbers in the .hermes/uml traces (the public
docs/architecture files drop line refs entirely, so nothing in the public
docs was affected).

- [2026-09-19] .hermes/uml/07-config-resolution.md: key-call-site line refs drifted; current locations: `MultiConfig::resolve` crates/aphrodite/src/config.rs:305 (was 301), `env_bool` :18, `env_parse_warn` :33, `apply_port_override` :414 (was 410), `Config::load` crates/aphrodite/src/config_loader.rs:21 (was 23), `get_bool` :67 (was 75), `get_u64` :81 (was 89), `get_string` :101 (was 109), `apply_compression` :125 (was 133), `resolve_thresholds` crates/aphrodite/src/proxy.rs:121 (was 130), config-file watcher crates/aphrodite/src/main.rs:250-268 (was 251).
- [2026-09-19] .hermes/uml/08-dylib-hotreload.md: shim line refs drifted (crates/aphrodite/templates/**init**.py, byte-identical to plugins/aphrodite/**init**.py); current: `_load_dylib` :484 (was 492), `_load_fresh_copy` :282 (was 290), `_probe_dylib` :365 (was 373), `_hotreload_dir` :159 (was 163), `_reap_stale_hotreloads` :235 (was 243), `_register_atexit_cleanup` :1057 (was 1077), `_ensure_binaries` :793 (was 803), `_check_version_published` :741 (was 751), `_configure_ffi` :420 (was 428), `_manual_ffi_setup` :450 (was 458), `_REQUIRED_VOID_P` :54 (was 56), `_call_json` :626 (was 636).
- [2026-09-19] .hermes/uml/09-release-ci.md: workflow line refs drifted; current: Build.yml `Release` job :52 (was 53), `Build` matrix :72 (was 73), Windows Get-FileHash step :159 (was 154), unix shasum step :170 (was 165), `Finalize` :205 (correct); Publish.yml `Test` :55 (was 56), packaging guard :96-104 (was 94), `Publish-Headroom-Core` :106 (was 107), version-exists check :144 (was 143), `Publish-Aphrodite` :163 (was 164), tag-push publish `aphrodite` :197 (correct), `Publish-Hermes` :202 (was 203), tag-push publish `aphrodite-hermes` :235 (correct).
