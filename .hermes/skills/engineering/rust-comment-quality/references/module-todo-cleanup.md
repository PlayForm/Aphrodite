# Module TODO Cleanup - patterns and before/after examples

Reference for converting module-level `//! TODO:` markers into structured
documentation, from a pass over the Aphrodite `crates/aphrodite-hermes/src/`
tree (40+ Rust files across the CCR engine, proxy, context engine, directive
resolution, and misc modules).

## Patterns applied

1. **Stub placeholder headers** (e.g. proxy stubs, directive modules)
   Before:
    ```
    //! TODO: zero callers.
    ```
    After:
    ```
    //! Status: not yet wired; all exports are cfg-gated behind the `proxy`
    //! feature.
    ```
2. **Module doc TODOs** (CCR store, retrieval, search, directives)
   Before:
    ```
    //! TODO: zero callers. Pending wire-up from the engine's main entry.
    ```
    After:
    ```
    //! ## Status
    //!
    //! Zero callers. Pending wire-up from the engine's main entry.
    ```
3. **Batch TODO lists** (engine facade, proxy dispatcher)
   Before:
    ```
    // TODO: add crash recovery mechanism
    // TODO: implement startup progress indicator
    ```
    After:
    ```
    //! ## Planned Work
    //!
    //! - Crash recovery mechanism
    //! - Startup progress indicator
    ```
4. **Technical migration notes** (certificate renewal module, hot-reload
   loop)
   Before:
    ```
    //! TODO: the inner Mutex should become tokio::sync::Mutex...
    ```
    After:
    ```
    //! A future migration to `tokio::sync::Mutex` will let this function
    //! await the renewal directly.
    ```

## What was deliberately left alone

Inline `// TODO:` markers inside function bodies represent site-specific
next steps (a handful remaining across the proxy's connection handlers).
They instruct the next implementer of that specific function and belong
there.

## Approach

Batch-patch with the `patch` tool, avoiding logic changes; share patch hunks
across sibling files.
