"""Telemetry module - Observability for Loom agents.

This module provides OpenTelemetry integration:
- init_telemetry: Initialize tracing
- shutdown_telemetry: Graceful shutdown
"""

from .tracing import init_telemetry, shutdown_telemetry

__all__ = [
    "init_telemetry",
    "shutdown_telemetry",
]
