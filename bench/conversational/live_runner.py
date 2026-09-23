#!/usr/bin/env python3
"""Compatibility shim - the atomized live runner now lives in the live/ package.

The 295-line flat module was split into live/{task_prompt,agent,scenario,
stats,cli}.py. The Hermes-interpreter re-exec bootstrap lives in live/cli.py;
this shim keeps `python3 live_runner.py ...` working.
"""

from live.cli import main

if __name__ == "__main__":
    main()