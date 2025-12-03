"""CLI module - Command line interface for Loom.

This module provides the CLI entry points:
- loom init: Initialize a new project
- loom run: Run a project
- loom dev: Start development server
- loom proto: Generate protobuf stubs
"""

from .main import main

__all__ = ["main"]
