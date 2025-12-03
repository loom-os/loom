"""Bridge module - gRPC communication with Rust Core.

This module handles the Bridge protocol for connecting Python agents
to Rust Core:
- BridgeClient: gRPC client for Bridge service
- proto/: Generated protobuf code
"""

from .client import BridgeClient

# Re-export proto modules for convenience
from .proto import action_pb2, bridge_pb2, bridge_pb2_grpc, event_pb2, memory_pb2, memory_pb2_grpc

__all__ = [
    "BridgeClient",
    # Proto modules
    "action_pb2",
    "bridge_pb2",
    "bridge_pb2_grpc",
    "event_pb2",
    "memory_pb2",
    "memory_pb2_grpc",
]
