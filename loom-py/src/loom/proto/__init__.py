"""Protocol buffer Python modules package.

Generated stubs live in the subpackage ``loom.proto.generated`` after running
``loom proto`` in a monorepo checkout. Published wheels will vendor those files
so end users do **not** need ``grpcio-tools``.

Backward compatibility: we re-export common modules at this level so existing
imports like ``from loom.proto import bridge_pb2`` keep working.
"""

from importlib import import_module
from . import generated as _generated  # type: ignore

_NAMES = [
	"bridge_pb2",
	"bridge_pb2_grpc",
	"event_pb2",
	"action_pb2",
	"agent_pb2",
	"plugin_pb2",
]

for _name in _NAMES:
	try:
		globals()[_name] = import_module(f"loom.proto.generated.{_name}")
	except Exception:
		# Modules may not exist yet if user hasn't generated them; remain lazy.
		pass

__all__ = _NAMES
