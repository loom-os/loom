"""Generate Python gRPC stubs from the Loom .proto files.

Usage:
    python -m loom.proto.generate

Requires `grpcio-tools`.
"""
from pathlib import Path
import subprocess
import sys

# Attempt to locate repository root dynamically by searching upward for 'loom-proto' directory.
def _find_repo_root(start: Path) -> Path:
    cur = start
    for _ in range(10):  # reasonable ascent limit
        if (cur / "loom-proto" / "proto").exists():
            return cur
        if cur.parent == cur:
            break
        cur = cur.parent
    raise RuntimeError("Could not locate repository root containing 'loom-proto/proto'")

REPO_ROOT = _find_repo_root(Path(__file__).resolve())
PROTO_SRC = REPO_ROOT / "loom-proto" / "proto"
OUT_DIR = Path(__file__).resolve().parent / "generated"
OUT_DIR.mkdir(parents=True, exist_ok=True)

FILES = [
    "bridge.proto",
    "event.proto",
    "action.proto",
    "agent.proto",
    "plugin.proto",
]

def main():
    if not PROTO_SRC.exists():
        print(f"Proto source dir not found: {PROTO_SRC}", file=sys.stderr)
        sys.exit(1)
    cmd = [
        sys.executable,
        "-m",
        "grpc_tools.protoc",
        f"-I{PROTO_SRC}",
        f"--python_out={OUT_DIR}",
        f"--grpc_python_out={OUT_DIR}",
    ] + FILES
    print("Generating stubs:", " ".join(FILES))
    subprocess.check_call(cmd, cwd=PROTO_SRC)
    # Patch imports (relative -> absolute) if necessary
    for py in OUT_DIR.glob("*_pb2*.py"):
        text = py.read_text()
        lines = []
        for line in text.splitlines():
            # Normalize absolute imports to relative
            if line.startswith("import ") and "_pb2" in line and "from ." not in line:
                parts = line.split()
                if len(parts) >= 2:
                    target = parts[1]
                    line = f"from . import {target}"
            # Add alias expected by grpc python plugin (action__pb2, bridge__pb2, etc.)
            if line.startswith("from . import ") and line.strip().endswith("_pb2") and " as " not in line:
                mod = line.strip().split()[-1]
                alias = mod.replace("_pb2", "__pb2")
                line = f"from . import {mod} as {alias}"
            lines.append(line)
        patched = "\n".join(lines)
        if patched != text:
            py.write_text(patched)
    print("Done.")

if __name__ == "__main__":
    main()
