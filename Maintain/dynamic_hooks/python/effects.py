"""
Aphrodite Effect Runtime - Effect-TS pattern for Python.

Core concepts (from Effect-TS):
  Effect<A, E, R>  = immutable description of a computation
                      A = success type, E = error type, R = required services
  pipe(value, f1, f2, ...)  = left-to-right composition
  Runtime  = provides services, executes effects at the edge

  An Effect is a VALUE, not a function. It doesn't run until Runtime.run().
  Extensions compose effects with .map() / .flatMap() / pipe().
  The dylib is a SERVICE - effects declare it as a requirement.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Callable, Generic, TypeVar

# ── Type variables ──────────────────────────────────────────────────────────
A = TypeVar("A")  # Success
E = TypeVar("E")  # Error
R = TypeVar("R")  # Requirements (services needed)
B = TypeVar("B")


# ── Service / Context ───────────────────────────────────────────────────────
#   A Service is a named value provided by the Runtime.
#   Effects declare service needs via Effect.service("name").
#   Extensions register services via Runtime.provide("name", value_or_factory).


class ServiceRegistry:
    """Holds named services provided to effects during execution."""

    def __init__(self) -> None:
        self._services: dict[str, Any] = {}

    def provide(self, name: str, value: Any) -> ServiceRegistry:
        self._services[name] = value
        return self

    def get(self, name: str) -> Any:
        if name not in self._services:
            raise KeyError(f"Service '{name}' not found. Available: {list(self._services)}")
        return self._services[name]

    def has(self, name: str) -> bool:
        return name in self._services


# ── Effect ──────────────────────────────────────────────────────────────────
#   Effect<A, E, R> describes a computation that:
#     - succeeds with value of type A
#     - fails with error of type E
#     - requires services in R (a frozenset of service names)


@dataclass(frozen=True)
class Effect(Generic[A]):
    """
    An immutable description of a computation.

    Create effects with:
      Effect.succeed(value)         - always succeeds
      Effect.fail(error)            - always fails
      Effect.sync(fn)               - sync computation that may throw
      Effect.service("dylib")       - accesses a service
      Effect.from_callable(fn)      - wraps a callable returning (result, error)
    """

    _run: Callable[[ServiceRegistry], tuple[Any, Any | None]] = field(repr=False)
    # _run returns (success_value, None) or (None, error_value)

    @staticmethod
    def succeed(value: B) -> Effect[B]:
        """Effect that always succeeds with the given value."""
        return Effect(lambda _services: (value, None))

    @staticmethod
    def fail(error: Any) -> Effect[Any]:
        """Effect that always fails with the given error."""
        return Effect(lambda _services: (None, error))

    @staticmethod
    def sync(fn: Callable[[], B]) -> Effect[B]:
        """Effect wrapping a synchronous computation. Throws become failures."""

        def _run(services: ServiceRegistry) -> tuple[Any, Any | None]:
            try:
                return (fn(), None)
            except Exception as exc:
                return (None, exc)

        return Effect(_run)

    @staticmethod
    def try_(fn: Callable[[], B], on_error: Callable[[Exception], Any] | None = None) -> Effect[B]:
        """Effect wrapping a fallible sync computation with custom error mapping."""

        def _run(services: ServiceRegistry) -> tuple[Any, Any | None]:
            try:
                return (fn(), None)
            except Exception as exc:
                err = on_error(exc) if on_error else exc
                return (None, err)

        return Effect(_run)

    @staticmethod
    def service(name: str) -> Effect[Any]:
        """Effect that accesses a named service from the runtime."""

        def _run(services: ServiceRegistry) -> tuple[Any, Any | None]:
            try:
                return (services.get(name), None)
            except KeyError as exc:
                return (None, exc)

        return Effect(_run)

    @staticmethod
    def from_callable(fn: Callable[..., Any], *args: Any, **kwargs: Any) -> Effect[Any]:
        """Effect wrapping an arbitrary callable."""

        def _run(services: ServiceRegistry) -> tuple[Any, Any | None]:
            try:
                return (fn(*args, **kwargs), None)
            except Exception as exc:
                return (None, exc)

        return Effect(_run)

    # ── Composition ─────────────────────────────────────────────────────

    def map(self, f: Callable[[A], B]) -> Effect[B]:
        """Transform the success value."""

        def _run(services: ServiceRegistry) -> tuple[Any, Any | None]:
            value, error = self._run(services)
            if error is not None:
                return (None, error)
            try:
                return (f(value), None)
            except Exception as exc:
                return (None, exc)

        return Effect(_run)

    def flat_map(self, f: Callable[[A], Effect[B]]) -> Effect[B]:
        """Chain effects: run this, feed success into f, run the result."""

        def _run(services: ServiceRegistry) -> tuple[Any, Any | None]:
            value, error = self._run(services)
            if error is not None:
                return (None, error)
            return f(value)._run(services)

        return Effect(_run)

    def catch_all(self, f: Callable[[Any], Effect[B]]) -> Effect[B]:
        """Recover from errors: if this fails, run the recovery effect."""

        def _run(services: ServiceRegistry) -> tuple[Any, Any | None]:
            value, error = self._run(services)
            if error is not None:
                return f(error)._run(services)
            return (value, None)

        return Effect(_run)

    def provide_service(self, name: str, value: Any) -> Effect[A]:
        """Provide a service to this effect (eliminates a requirement)."""

        def _run(services: ServiceRegistry) -> tuple[Any, Any | None]:
            scoped = ServiceRegistry()
            # Copy existing services + add the new one
            for k, v in services._services.items():
                scoped._services[k] = v
            scoped._services[name] = value
            return self._run(scoped)

        return Effect(_run)

    def tap(self, f: Callable[[A], Any]) -> Effect[A]:
        """Run a side-effect for observation, pass the value through."""

        def _run(services: ServiceRegistry) -> tuple[Any, Any | None]:
            value, error = self._run(services)
            if error is not None:
                return (None, error)
            try:
                f(value)
            except Exception:
                pass  # tap failures are silently ignored
            return (value, None)

        return Effect(_run)

    # ── Execution ───────────────────────────────────────────────────────

    def run_sync(self, services: ServiceRegistry | None = None) -> A:
        """Execute synchronously. Returns value or raises error."""
        svc = services or ServiceRegistry()
        value, error = self._run(svc)
        if error is not None:
            if isinstance(error, Exception):
                raise error
            raise RuntimeError(str(error))
        return value

    def run_sync_exit(self, services: ServiceRegistry | None = None) -> dict:
        """Execute and return {'_tag': 'Success', 'value': ...} or {'_tag': 'Failure', 'error': ...}."""
        svc = services or ServiceRegistry()
        value, error = self._run(svc)
        if error is not None:
            return {"_tag": "Failure", "error": error}
        return {"_tag": "Success", "value": value}


# ── pipe ────────────────────────────────────────────────────────────────────
#   Left-to-right composition. Each function receives the previous result.
#   Designed to match Effect-TS: pipe(value, Effect.map(f), Effect.flatMap(g), ...)


def pipe(initial: Any, *fns: Callable[[Any], Any]) -> Any:
    """
    Thread a value through a sequence of transformations.

    Usage:
      pipe(
          "hello",
          lambda s: s.upper(),         # "HELLO"
          lambda s: s + " world",      # "HELLO world"
          lambda s: len(s),            # 11
      )

      pipe(
          Effect.succeed(42),
          lambda e: e.map(lambda x: x + 1),
          lambda e: e.run_sync(),
      )  # → 43
    """
    result = initial
    for fn in fns:
        result = fn(result)
    return result


# ── Runtime ─────────────────────────────────────────────────────────────────
#   The runtime is the single source of services + pipeline registry.
#   Extensions register services and pipeline effects here.


class Runtime:
    """
    Central registry for services and hook pipelines.

    Built-in services (provided at bootstrap):
      "dylib"       - loaded dylib handle (via ctypes.CDLL)
      "config"      - configuration dict

    Extensions can register additional services and pipeline effects.
    """

    def __init__(self) -> None:
        self._services = ServiceRegistry()
        self._pipelines: dict[str, list[Effect]] = {}  # hook_name → [Effect, ...]

    # ── Services ────────────────────────────────────────────────────────

    def provide(self, name: str, value: Any) -> Runtime:
        """Register a service. Fluent - returns self."""
        self._services.provide(name, value)
        return self

    def service(self, name: str) -> Any:
        return self._services.get(name)

    # ── Pipeline registry ───────────────────────────────────────────────

    def pipeline(self, hook_name: str, effects: list[Callable[[Any], Effect]]) -> Runtime:
        """Define the effect pipeline for a Hermes hook.

        Each element is a function: (prev_value) -> Effect
        The runtime chains them via flat_map at execution time.
        """
        self._pipelines[hook_name] = list(effects)
        return self

    def prepend(self, hook_name: str, effect_fn: Callable[[Any], Effect]) -> Runtime:
        if hook_name not in self._pipelines:
            self._pipelines[hook_name] = []
        self._pipelines[hook_name].insert(0, effect_fn)
        return self

    def append(self, hook_name: str, effect_fn: Callable[[Any], Effect]) -> Runtime:
        if hook_name not in self._pipelines:
            self._pipelines[hook_name] = []
        self._pipelines[hook_name].append(effect_fn)
        return self

    # ── Execution ───────────────────────────────────────────────────────

    def run(self, hook_name: str, input_data: Any) -> Any:
        """
        Execute the pipeline for a hook.

        Pipeline elements are functions: (prev_value) -> Effect.
        They're chained via flat_map: input_data → fn1 → fn2 → fn3 → result.

        Returns the final success value or raises the first error.
        """
        pipeline_fns = self._pipelines.get(hook_name, [])
        if not pipeline_fns:
            return input_data

        # Start with a succeeding effect wrapping the input
        effect: Effect = Effect.succeed(input_data)

        # Chain each pipeline function via flat_map
        for fn in pipeline_fns:
            effect = effect.flat_map(fn)

        return effect.run_sync(self._services)

    def run_exit(self, hook_name: str, input_data: Any) -> dict:
        """Execute and return Exit dict (never raises)."""
        try:
            result = self.run(hook_name, input_data)
            return {"_tag": "Success", "value": result}
        except Exception as exc:
            return {"_tag": "Failure", "error": exc}

    # ── Introspection ───────────────────────────────────────────────────

    def list_pipelines(self) -> dict[str, int]:
        return {name: len(effects) for name, effects in self._pipelines.items()}

    def list_services(self) -> list[str]:
        return list(self._services._services.keys())


# ── Singleton ──────────────────────────────────────────────────────────────
runtime = Runtime()
