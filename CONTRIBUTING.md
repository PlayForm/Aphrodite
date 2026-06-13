# Contributing

We welcome contributions to **@playform/hermes-compress**!

## Development Setup

```bash
git clone https://github.com/PlayForm/HermesCompress.git
cd HermesCompress
pip install -e ".[dev]"
```

## Project Conventions

### PascalCase

All exported symbols use **PascalCase** - this is the PlayForm convention
carried over from the TypeScript origin (`@playform/compress`).

### Module Layout

```
Source/
├── __init__.py       # Plugin entry, register(ctx)
├── Compress.py       # Main engine
├── Function/         # Utilities (Bytes, Directory, Integration, Merge)
├── Interface/        # Dataclasses (CompressOption)
└── Variable/         # Defaults (per-tool hints, parser routing)
```

### Hermes Plugin Rules

- `plugin.yaml` must declare `kind`, `provides_tools`, `provides_hooks`.
- `register(ctx)` is the single entry point called by the Hermes plugin manager.
- Tools are registered via `ctx.register_tool()`, hooks via
  `ctx.register_hook()`.

### Testing

```bash
pytest Source/ -v
```

## Pull Requests

1. Fork the repository.
2. Create a feature branch.
3. Make your changes, following the conventions above.
4. Add or update tests.
5. Update `CHANGELOG.md`.
6. Submit a PR against the `Current` branch.

## License

By contributing, you agree that your contributions will be licensed under the
Apache 2.0 License.
