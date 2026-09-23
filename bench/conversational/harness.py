"""Compatibility shim - the atomized harness now lives in the harness/ package.

The 1069-line flat module was split into harness/{provider,scenarios,results,
proxy_manager,provider_client,proxy_client,tokens,runner,binary,run,paths}.py
(each one export, reverse-hierarchical; __init__.py re-exports every public
name). This shim keeps `python3 harness.py ...` and `import harness` working.
"""

from harness.run import main

if __name__ == "__main__":
    main()
