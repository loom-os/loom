from __future__ import annotations
import argparse
import asyncio
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
    # Generate proto stubs from repo protos
    from .proto import generate  # type: ignore
    generate.main()


def cmd_dev(args):
    # Start the bridge server locally via cargo (best effort) and export LOOM_BRIDGE_ADDR
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
        ["cargo", "run", "-p", "loom-bridge", "--bin", "loom-bridge-server"],
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


def cmd_new(args):
    # Scaffold a minimal agent project
    target = Path(args.path).resolve()
    target.mkdir(parents=True, exist_ok=True)
    (target / "agent.py").write_text(
        """
from loom import Agent, capability

@capability("hello.echo", version="1.0")
def echo(text: str):
    return {"echo": text}

async def on_event(ctx, topic, event):
    await ctx.emit("topic.hello", type="hello", payload=event.payload)

agent = Agent("py-agent", topics=["topic.hello"], capabilities=[echo], on_event=on_event)

if __name__ == "__main__":
    agent.run()
""".strip()
    )
    print(f"[loom] Project created at {target}")


def cmd_run(args):
    # Run a user script with the current environment
    script = args.script
    if not Path(script).exists():
        print(f"[loom] Script not found: {script}")
        sys.exit(1)
    os.execv(sys.executable, [sys.executable, script])


def main():
    p = argparse.ArgumentParser(prog="loom", description="Loom Python SDK CLI")
    sub = p.add_subparsers(dest="cmd", required=True)

    sp = sub.add_parser("proto", help="Generate Python gRPC stubs from repo protos")
    sp.set_defaults(func=cmd_proto)

    sd = sub.add_parser("dev", help="Start local Loom bridge server (requires cargo)")
    sd.add_argument("--port", type=int, default=None)
    sd.set_defaults(func=cmd_dev)

    sn = sub.add_parser("new", help="Scaffold a minimal agent project")
    sn.add_argument("path", nargs="?", default=".")
    sn.set_defaults(func=cmd_new)

    sr = sub.add_parser("run", help="Run a Python script in the current environment")
    sr.add_argument("script")
    sr.set_defaults(func=cmd_run)

    args = p.parse_args()
    args.func(args)

if __name__ == "__main__":
    main()
