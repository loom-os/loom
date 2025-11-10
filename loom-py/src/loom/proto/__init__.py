"""Protocol buffer Python modules.

Wheels published to PyPI should include generated files *_pb2.py and *_pb2_grpc.py
within this package directory so runtime doesn't need grpcio-tools.

When developing inside the monorepo, run `loom proto` to (re)generate them from
`../loom-proto/proto/*.proto`.
"""

__all__ = []
