from __future__ import annotations

import hashlib
import os
import platform
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path
from typing import Optional
from urllib.request import urlopen

from platformdirs import user_cache_dir

VENDOR = "loom-os"
APP_NAME = "loom"
GITHUB_REPO = "loom-os/loom"
RELEASE_BASE_URL = f"https://github.com/{GITHUB_REPO}/releases/download"


def platform_tag() -> str:
    """Return platform tag for binary selection (e.g., linux-x86_64, macos-aarch64)."""
    sysname = sys.platform
    arch = platform.machine().lower()
    # Normalize common arch names
    if arch in ("x86_64", "amd64"):
        arch = "x86_64"
    elif arch in ("aarch64", "arm64"):
        arch = "aarch64"

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
    """Return cache directory for downloaded binaries."""
    return Path(user_cache_dir(APP_NAME, VENDOR)) / "bin"


def binary_path(binary_name: str, version: str = "latest") -> Path:
    """Return path to cached binary for given version and platform."""
    tag = platform_tag()
    name = binary_name + (".exe" if sys.platform.startswith("win") else "")
    return cache_dir() / version / tag / name


def ensure_executable(p: Path) -> None:
    """Make file executable on Unix systems."""
    if sys.platform.startswith("win"):
        return
    mode = p.stat().st_mode
    p.chmod(mode | stat.S_IEXEC | stat.S_IREAD | stat.S_IWRITE)


def verify_checksum(file_path: Path, expected_sha256: Optional[str]) -> bool:
    """Verify SHA256 checksum of downloaded file."""
    if not expected_sha256:
        return True  # Skip verification if no checksum provided

    sha256 = hashlib.sha256()
    with open(file_path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            sha256.update(chunk)

    actual = sha256.hexdigest()
    return actual.lower() == expected_sha256.lower()


def download_from_github(binary_name: str, version: str) -> Path:
    """Download binary from GitHub Releases.

    Expected release asset format:
    - {binary_name}-{version}-{platform_tag}.tar.gz (Linux/macOS)
    - {binary_name}-{version}-{platform_tag}.zip (Windows)

    Optional checksum file: {asset_name}.sha256
    """
    tag = platform_tag()
    is_windows = sys.platform.startswith("win")
    ext = "zip" if is_windows else "tar.gz"
    asset_name = f"{binary_name}-{version}-{tag}.{ext}"
    asset_url = f"{RELEASE_BASE_URL}/v{version}/{asset_name}"
    checksum_url = f"{asset_url}.sha256"

    print(f"[loom] Downloading {binary_name} v{version} for {tag}...")
    print(f"[loom] URL: {asset_url}")

    # Download archive
    try:
        with urlopen(asset_url) as response:
            archive_data = response.read()
    except Exception as e:
        raise RuntimeError(f"Failed to download {asset_name}: {e}") from e

    # Download and verify checksum (optional)
    expected_checksum = None
    try:
        with urlopen(checksum_url) as response:
            checksum_text = response.read().decode("utf-8").strip()
            # Format: "<hash>  <filename>" or just "<hash>"
            expected_checksum = checksum_text.split()[0]
    except Exception:
        print("[loom] Warning: Checksum file not found, skipping verification")

    # Write archive to temp file
    with tempfile.NamedTemporaryFile(delete=False, suffix=f".{ext}") as tmp:
        tmp.write(archive_data)
        tmp_path = Path(tmp.name)

    try:
        # Verify checksum
        if expected_checksum:
            if not verify_checksum(tmp_path, expected_checksum):
                raise RuntimeError(f"Checksum verification failed for {asset_name}")
            print("[loom] Checksum verified")

        # Extract binary
        extract_dir = cache_dir() / version / tag
        extract_dir.mkdir(parents=True, exist_ok=True)

        if is_windows:
            with zipfile.ZipFile(tmp_path, "r") as zf:
                zf.extractall(extract_dir)
        else:
            with tarfile.open(tmp_path, "r:gz") as tf:
                tf.extractall(extract_dir)

        # Find and return binary path
        binary_file = binary_path(binary_name, version)
        if not binary_file.exists():
            raise FileNotFoundError(f"Binary not found after extraction: {binary_file}")

        ensure_executable(binary_file)
        print(f"[loom] Downloaded and cached at {binary_file}")
        return binary_file

    finally:
        tmp_path.unlink(missing_ok=True)


def find_local_build(binary_name: str) -> Optional[Path]:
    """Try to locate locally built binary (dev convenience)."""
    candidates = [
        Path(f"target/debug/{binary_name}"),
        Path(f"target/release/{binary_name}"),
        Path(f"bridge/target/debug/{binary_name}"),
        Path(f"bridge/target/release/{binary_name}"),
        Path(f"core/target/debug/{binary_name}"),
        Path(f"core/target/release/{binary_name}"),
    ]
    for c in candidates:
        if c.exists():
            return c
    return None


def get_binary(binary_name: str, version: str = "latest", allow_local: bool = True) -> Path:
    """Get binary, downloading if necessary or using local build in dev mode."""
    # Check cache first
    cached = binary_path(binary_name, version)
    if cached.exists():
        return cached

    # Try local build (dev convenience)
    if allow_local:
        local = find_local_build(binary_name)
        if local:
            print(f"[loom] Using local build: {local}")
            # Copy to cache for consistency
            cached.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(local, cached)
            ensure_executable(cached)
            return cached

    # Download from GitHub Releases
    return download_from_github(binary_name, version)


def start_binary(
    binary_name: str,
    env_vars: Optional[dict[str, str]] = None,
    version: str = "latest",
    allow_local: bool = True,
) -> subprocess.Popen:
    """Start a Loom runtime binary with given environment variables."""
    binary = get_binary(binary_name, version=version, allow_local=allow_local)
    env = os.environ.copy()
    if env_vars:
        env.update(env_vars)
    proc = subprocess.Popen([str(binary)], env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    return proc


# Convenience functions for specific binaries
def start_bridge(address: str, version: str = "latest") -> subprocess.Popen:
    """Start loom-bridge-server."""
    return start_binary(
        "loom-bridge-server",
        env_vars={"LOOM_BRIDGE_ADDR": address},
        version=version,
    )


def start_core(
    bridge_addr: str,
    dashboard_port: int = 3030,
    version: str = "latest",
) -> subprocess.Popen:
    """Start loom-core (full runtime with dashboard)."""
    return start_binary(
        "loom-core",
        env_vars={
            "LOOM_BRIDGE_ADDR": bridge_addr,
            "LOOM_DASHBOARD": "true",
            "LOOM_DASHBOARD_PORT": str(dashboard_port),
        },
        version=version,
    )
