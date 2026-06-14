"""
hermes-compress Hermes Agent plugin entry point.

Symlinked from ~/.hermes/plugins/hermes-compress/ → this directory.
Re-exports register() from the hermes_compress package so Hermes can
discover and load the plugin.
"""

from hermes_compress import register, __version__ # type: ignore

__all__ = ["register", "__version__"]
