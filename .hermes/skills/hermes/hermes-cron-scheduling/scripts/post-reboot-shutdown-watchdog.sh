#!/bin/bash
# Post-reboot shutdown watchdog (see hermes-cron-scheduling SKILL.md).
# Fires once: when the machine has been freshly booted (uptime 12-25 min)
# after an update reboot, shut it down via System Events. Marker file makes
# this one-shot so a later manual reboot never re-triggers.
# Register: hermes cron create 'every 5m' --name <job> --no-agent \
#   --script post-reboot-shutdown-watchdog.sh --deliver origin
MARKER="$HOME/.hermes/scripts/.post-reboot-shutdown.done"
[ -f "$MARKER" ] && exit 0

boot_epoch=$(sysctl -n kern.boottime | awk '{print $4}' | tr -d ',')
now=$(date +%s)
up=$((now - boot_epoch))

if [ "$up" -ge 720 ] && [ "$up" -le 1500 ]; then
  touch "$MARKER"
  osascript -e 'tell app "System Events" to shut down' 2>&1
  echo "Machine shutting down now (uptime ${up}s) - post-update."
else
  exit 0
fi