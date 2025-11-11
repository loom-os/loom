from __future__ import annotations
import os
import platform
import shutil
import stat
import subprocess
import sys
from pathlib import Path
from typing import Optional

from platformdirs import user_cache_dir

BIN_NAME = "loom-bridge-server"
VENDOR = "loom-os"
APP_NAME = "loom"


def platform_tag() -> str:
    sysname = sys.platform
    arch = platform.machine().lower()
    if sysname.startswith("linux"):
        os_tag = "linux"
    elif sysname == "darwin":
        os_tag = "macos"
    elif sysname.startswith("win"):
        os_tag = "windows"
    else:
        os_tag = sysname
    return f"{os_tag}-{arch}"


def cache_dir() -> Path:
    return Path(user_cache_dir(APP_NAME, VENDOR)) / "bin"


def core_path(version: str = "latest") -> Path:
    tag = platform_tag()
    name = BIN_NAME + (".exe" if sys.platform.startswith("win") else "")
    return cache_dir() / f"{name}-{version}-{tag}"


def ensure_executable(p: Path) -> None:
    if sys.platform.startswith("win"):
        return
    mode = p.stat().st_mode
    p.chmod(mode | stat.S_IEXEC)


def download_core(version: str = "latest") -> Path:
    """Placeholder: in a future revision, download from Releases and verify checksum.

    Today, we try to find a local build via cargo (debug bin) and copy it to cache as a stand-in.
    """
    # Try to locate cargo-built binary in repo (dev convenience)
    candidates = [
        Path("target/debug/" + BIN_NAME),
        Path("bridge/target/debug/" + BIN_NAME),
    ]
    for c in candidates:
        if c.exists():
            dst = core_path(version)
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(c, dst)
            ensure_executable(dst)
            return dst
    raise FileNotFoundError(
        "Embedded core binary not found. Build it with `cargo build -p loom-bridge --bin loom-bridge-server` "
        "or provide a downloadable URL in a future release."
    )


def start_core(address: str, version: str = "latest") -> subprocess.Popen:
    p = core_path(version)
    if not p.exists():
        p = download_core(version)
    env = os.environ.copy()
    env["LOOM_BRIDGE_ADDR"] = address
    proc = subprocess.Popen([str(p)], env=env)
    return proc
