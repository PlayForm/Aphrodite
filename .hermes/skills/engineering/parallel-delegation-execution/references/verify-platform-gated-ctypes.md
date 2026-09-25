# Verifying platform-gated ctypes code on a non-Windows host

Recipe for proving the logic of a `ctypes.windll` branch (e.g. a win32 PID
probe in the Aphrodite repo's platform-gated code) on macOS/Linux, where
the real API cannot run. Used during the verification step of fix waves
that touch platform-gated code - see the "platform-gated ctypes fix waves"
pitfall in this skill's SKILL.md.

## Why naive fakes fail

The code under test assigns `k32.OpenProcess.argtypes = [...]` before
calling - only real `ctypes._FuncPtr` objects support that. A fake built
from class methods fails twice: accessing an instance method returns a
BOUND method, and bound methods reject attribute assignment
(`AttributeError: 'method' object has no attribute 'argtypes'`); a fake
using `self.X = self._x` stores the bound method in the instance dict and
fails the same way. A fake using class functions stored unbound then fails
at CALL time (`TypeError: ... missing 1 required positional argument:
'self'`). Every failure is swallowed by the branch's defensive except, so
the harness reports all-True - indistinguishable from the code working.

## The working shape: closure-based plain functions

`ctypes.windll` does not exist on macOS - install it as a module attribute
before forcing the platform, and restore afterward:

```python
import ctypes, types, sys
state = {"open_returns": [], "last_error": 0, "closed": [], "exit_codes": {}}

def OpenProcess(access, inherit, pid):
    return state["open_returns"].pop(0)

def GetExitCodeProcess(h, byref_code):
    code = state["exit_codes"].get(h, 259)
    ctypes.cast(byref_code, ctypes.POINTER(ctypes.c_uint32))[0] = code
    return True

def CloseHandle(h):
    state["closed"].append(h)
    return True

def GetLastError():
    return state["last_error"]

ctypes.windll = types.SimpleNamespace(kernel32=types.SimpleNamespace(
    OpenProcess=OpenProcess, GetExitCodeProcess=GetExitCodeProcess,
    CloseHandle=CloseHandle, GetLastError=GetLastError))
sys.platform = "win32"
# ... call the function under test ...
sys.platform = saved_platform
del ctypes.windll
```

Plain module-level functions (or closures) assigned into a
SimpleNamespace are the ONLY shape that accepts `.argtypes`/`.restype`
assignment and takes the exact call arguments.

## Coverage to assert

- NULL handle + `ERROR_ACCESS_DENIED` (5) -> alive (True)
- NULL handle + other error (e.g. 87) -> dead (False)
- valid handle + `STILL_ACTIVE` (259) -> alive
- valid handle + exit code 0 -> dead
- `GetExitCodeProcess` returning False -> defensive True
- exception inside the branch -> warning logged + True (never a crash)
- `CloseHandle` called for every opened handle (no leaks)

A harness that cannot reach the success paths (every call logs the
defensive warning) is a fake-shape bug, not a code bug - fix the fake
before judging the implementation. In the Aphrodite repo, keep the
harness and its coverage list in the plan file for the wave so the
verification is reproducible after the fix lands.
