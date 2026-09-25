# Fleet version check - diagnosis & clear recipe

## Background

`hermes update` prints a `Fleet version check:` block (hermes_cli/update_receipt.py →
`collect_fleet_versions()` + `print_fleet_version_matrix()`). One row per profile that has a
`gateway_state.json` or a live control socket. The `? <profile> (pid N) - version unknown
(gateway predates version stamping; restart to enable)` row means the profile record claims a
running gateway but has no `code_sha`/`code_version` stamp.

## The PID-reuse trap (most common cause of phantom rows)

A `gateway_state.json` recorded a gateway PID that later DIED. An unrelated process reused that
PID (observed in the field: a security/antivirus helper process). `_pid_exists()` uses
`os.kill(pid, 0)`, which succeeds against the reused PID → probe thinks the gateway is alive →
emits the `?` row. Reality: no hermes gateway is running.

## Diagnosis commands

```
hermes gateway list                       # all profiles + running flag (all were "not running" in the repro)
hermes gateway status                     # default → "✗ Gateway is not running" + "Stale gateway_state.json" warning
hermes -p <profile> gateway status        # named profile (e.g. hermes -p dev-aphrodite gateway status)
launchctl list | grep -i hermes           # macOS: NO hermes service installed → nothing auto-restarts
ps -p <pid_from_row> -o pid,command       # command was NOT hermes (security helper) → confirms PID reuse
```

Decisive phantom signals: not running + no service + PID belongs to a non-hermes process.

## Enumerate affected state files

```
find ~/.hermes -name gateway_state.json -maxdepth 3
# ~/.hermes/gateway_state.json                                  (default, recorded a reused PID)
# ~/.hermes/profiles/<worker-profile>/gateway_state.json        (recorded the same reused PID)
# ~/.hermes/profiles/<profile-b>/gateway_state.json             (pid dead, NOT reused → skipped silently)
# ~/.hermes/profiles/<profile-c>/gateway_state.json             (pid dead, not reused → skipped)
# ~/.hermes/profiles/dev-aphrodite/gateway_state.json           (pid dead, not reused → skipped)
```

Note: profiles whose dead PIDs were never reused produce NO row - that's why only the
reused-PID profiles (default, `<worker-profile>`) appeared.

## Cleared (after) state-file shape

default (`~/.hermes/gateway_state.json`) and `<worker-profile>`:

```json
{
	"gateway_state": "stopped",
	"exit_reason": "manual-clear-stale",
	"restart_requested": false,
	"pid": null,
	"active_agents": 0,
	"platforms": {},
	"updated_at": "<now ISO-8601>"
}
```

`<worker-profile>` also kept its `"hermes_home": "$HOME/.hermes/profiles/<worker-profile>"` key.

## Verification (exact repro)

```python
from hermes_cli import update_receipt as ur
fleet = ur.collect_fleet_versions()
print("rows:", len(fleet))          # 0 after clear
ur.print_fleet_version_matrix(fleet)
```

`hermes gateway status` then shows "not running" with "Last shutdown reason: manual-clear-stale".
Backups of the originals were written to `~/.hermes/tmp/hgw-backup/` (repo scratch convention -
never `/tmp`).
