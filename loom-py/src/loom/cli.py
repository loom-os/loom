from __future__ import annotations
import argparse
import os
import shutil
import signal
import socket
import subprocess
import sys
from pathlib import Path


def _pick_free_port() -> int:
    s = socket.socket()
    s.bind(("127.0.0.1", 0))
    addr, port = s.getsockname()
    s.close()
    return port


def cmd_proto(args):
    """Generate gRPC stubs into proto/generated/ (dev workflow)."""
    from .proto import generate  # type: ignore
    generate.main()


def cmd_dev(args):
    """Start the bridge server locally via cargo and export LOOM_BRIDGE_ADDR."""
    cargo = shutil.which("cargo")
    port = args.port or _pick_free_port()
    addr = f"127.0.0.1:{port}"
    env = os.environ.copy()
    env["LOOM_BRIDGE_ADDR"] = addr
    print(f"[loom] Starting loom-bridge at {addr} ...")
    if not cargo:
        print("[loom] 'cargo' not found. Please start the bridge server manually or install Rust.")
        print("      cargo run -p loom-bridge --bin loom-bridge-server")
        sys.exit(2)
    proc = subprocess.Popen(
        [cargo, "run", "-p", "loom-bridge", "--bin", "loom-bridge-server"],
        env=env,
    )
    print("[loom] Press Ctrl+C to stop.")
    try:
        proc.wait()
    except KeyboardInterrupt:
        proc.send_signal(signal.SIGINT)
        try:
            proc.wait(timeout=5)
        except subprocess.TimeoutExpired:
            proc.kill()


TEMPLATE_AGENT = '''from loom import Agent, capability
import asyncio

@capability("hello.echo", version="1.0")
def echo(text: str):
    return {"echo": text}

async def on_event(ctx, topic, event):
    if event.type == "user.message":
        await ctx.emit("topic.hello", type="hello", payload=event.payload)

agent = Agent("py-agent", topics=["topic.hello"], capabilities=[echo], on_event=on_event)

if __name__ == "__main__":
    agent.run()
'''.strip()

TEMPLATE_CONFIG = '''# loom project config
topics = ["topic.hello"]
# future options: managed_endpoint = "bridge.loomcloud.dev:443"
'''.strip()


def cmd_init(args):
    target = Path(args.path).resolve()
    target.mkdir(parents=True, exist_ok=True)
    (target / "agent.py").write_text(TEMPLATE_AGENT)
    (target / "loom.toml").write_text(TEMPLATE_CONFIG)
    print(f"[loom] Initialized project at {target}")


def cmd_run(args):
    """Run a user script with the current environment."""
    script = args.script
    if not Path(script).exists():
        print(f"[loom] Script not found: {script}")
        sys.exit(1)
    os.execv(sys.executable, [sys.executable, script])


def _load_project_config(start: Path) -> dict:
    """Load loom.toml using tomllib (py>=3.11) or tomli; return {} if missing/invalid."""
    cfg_path = start / "loom.toml"
    if not cfg_path.exists():
        return {}
    if sys.version_info >= (3, 11):
        import tomllib as toml  # type: ignore
    else:
        try:
            import tomli as toml  # type: ignore
        except Exception:
            return {}
    try:
        return toml.loads(cfg_path.read_text(encoding="utf-8"))
    except Exception:
        return {}


def _toml_format_value(v):
    """Minimal TOML value formatter for strings, numbers, bools, and simple lists."""
    if isinstance(v, bool):
        return "true" if v else "false"
    if isinstance(v, (int, float)):
        return str(v)
    if isinstance(v, str):
        esc = v.replace("\\", "\\\\").replace("\"", "\\\"")
        return f'"{esc}"'
    if isinstance(v, (list, tuple)):
        return "[" + ", ".join(_toml_format_value(x) for x in v) + "]"
    return f'"{str(v)}"'


def _toml_dumps_minimal(cfg: dict) -> str:
    """Dump a minimal TOML supporting top-level keys and one-level tables."""
    lines: list[str] = []
    # top-level keys
    for k in sorted(cfg.keys()):
        v = cfg[k]
        if not isinstance(v, dict):
            lines.append(f"{k} = {_toml_format_value(v)}")
    if lines:
        lines.append("")
    # tables
    for k in sorted(cfg.keys()):
        v = cfg[k]
        if isinstance(v, dict):
            lines.append(f"[{k}]")
            for sk in sorted(v.keys()):
                sv = v[sk]
                lines.append(f"{sk} = {_toml_format_value(sv)}")
            lines.append("")
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines) + "\n"


def _write_project_bridge(start: Path, address: str, mode: str, version: str):
    """Merge bridge config while preserving existing keys."""
    existing = _load_project_config(start)
    bridge = existing.get("bridge") or {}
    bridge.update({"address": address, "mode": mode, "version": version})
    existing["bridge"] = bridge
    (start / "loom.toml").write_text(_toml_dumps_minimal(existing), encoding="utf-8")


def cmd_up(args):
    """Start (or reuse) embedded core binary and export LOOM_BRIDGE_ADDR.

    Tries to start cached binary; if missing, attempts local dev build copy.
    """
    from . import embedded
    version = args.version
    port = _pick_free_port()
    address = f"127.0.0.1:{port}"
    proc = embedded.start_core(address, version=version)
    os.environ["LOOM_BRIDGE_ADDR"] = address
    _write_project_bridge(Path("."), address, "embedded", version)
    print(f"[loom] Embedded core started PID={proc.pid} at {address} (version {version})")


def main():
    p = argparse.ArgumentParser(prog="loom", description="Loom Python SDK CLI")
    sub = p.add_subparsers(dest="cmd", required=True)

    sp = sub.add_parser("proto", help="Generate Python gRPC stubs into proto/generated/")
    sp.set_defaults(func=cmd_proto)

    sd = sub.add_parser("dev", help="Start local Loom bridge server (requires cargo)")
    sd.add_argument("--port", type=int, default=None)
    sd.set_defaults(func=cmd_dev)

    si = sub.add_parser("init", help="Create a new Loom agent project in PATH (default .)")
    si.add_argument("path", nargs="?", default=".")
    si.set_defaults(func=cmd_init)

    sn = sub.add_parser("new", help="Alias for 'init'")
    sn.add_argument("path", nargs="?", default=".")
    sn.set_defaults(func=cmd_init)

    sc = sub.add_parser("create", help="Alias for 'init'")
    sc.add_argument("path", nargs="?", default=".")
    sc.set_defaults(func=cmd_init)

    sr = sub.add_parser("run", help="Run a Python script in the current environment")
    sr.add_argument("script")
    sr.set_defaults(func=cmd_run)

    su = sub.add_parser("up", help="Start or connect to embedded core, set LOOM_BRIDGE_ADDR")
    su.add_argument("--version", default="latest")
    su.set_defaults(func=cmd_up)

    args = p.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
