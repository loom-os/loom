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
    """Make file executable on Unix systems.

    Sets read and execute permissions for owner, group, and others.
    Does NOT set write permission for security (downloaded binaries should be read-only).
    """
    if sys.platform.startswith("win"):
        return
    mode = p.stat().st_mode
    # Set read and execute for owner, group, and others (0o555 = r-xr-xr-x)
    p.chmod(
        mode
        | stat.S_IRUSR
        | stat.S_IXUSR
        | stat.S_IRGRP
        | stat.S_IXGRP
        | stat.S_IROTH
        | stat.S_IXOTH
    )


def get_binary_version(binary_path: Path) -> Optional[str]:
    """Get version string from binary by calling --version.

    Returns None if binary doesn't support --version or execution fails.
    """
    try:
        result = subprocess.run(
            [str(binary_path), "--version"],
            capture_output=True,
            text=True,
            timeout=5,
            check=False,
        )
        if result.returncode == 0:
            # Extract version from output like "loom-bridge-server 0.1.0"
            output = result.stdout.strip()
            if output:
                parts = output.split()
                # Return last part which should be the version
                return parts[-1] if parts else None
        return None
    except (subprocess.TimeoutExpired, FileNotFoundError, Exception):
        return None


def validate_cached_binary(cached: Path, binary_name: str, expected_version: str) -> bool:
    """Validate that cached binary exists, is executable, and has correct version.

    Args:
        cached: Path to cached binary
        binary_name: Name of the binary (for logging)
        expected_version: Expected version string

    Returns:
        True if binary is valid, False otherwise
    """
    if not cached.exists():
        return False

    # Check if executable
    if not os.access(cached, os.X_OK):
        print(f"[loom] Cached binary not executable: {cached}")
        return False

    # Check version (skip for "latest" since we can't validate)
    if expected_version != "latest":
        actual_version = get_binary_version(cached)
        if actual_version is None:
            print(f"[loom] WARNING: Cannot determine version of cached binary: {cached}")
            print("[loom]   Removing cache and will download fresh binary")
            cached.unlink(missing_ok=True)
            return False

        if actual_version != expected_version:
            print(f"[loom] Version mismatch for {binary_name}:")
            print(f"[loom]   Expected: {expected_version}")
            print(f"[loom]   Cached:   {actual_version}")
            print("[loom]   Removing stale cache...")
            cached.unlink(missing_ok=True)
            return False

    return True


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
        # Checksum file is optional; if it doesn't exist or fails to download, skip verification
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


def find_local_build(binary_name: str, prefer_release: bool = True) -> Optional[Path]:
    """Try to locate locally built binary (dev convenience).

    This function searches for binaries built from source using `cargo build`.
    It's useful during development to avoid downloading binaries from GitHub.

    Search strategy:
    1. Check current directory and parents for Cargo.toml (repo root)
    2. Search in target/debug, target/release, bridge/target/*, core/target/*
    3. Prefer release over debug by default (configurable)

    To build locally:
        cd /path/to/loom/repo
        cargo build --release -p loom-bridge

    See docs/BUILD_LOCAL.md for complete build instructions.

    Args:
        binary_name: Name of binary to find
        prefer_release: If True, prefer release over debug builds (default: True)

    Returns:
        Path to binary if found, None otherwise
    """
    # Try to find repo root by looking for Cargo.toml with workspace members
    current = Path.cwd()
    repo_root = None

    # Search up to 5 levels up for the repo root
    for _ in range(5):
        cargo_toml = current / "Cargo.toml"
        if cargo_toml.exists():
            try:
                content = cargo_toml.read_text()
                if "[workspace]" in content or "members" in content:
                    repo_root = current
                    break
            except Exception:
                # Ignore read errors (permissions, encoding, etc.) and continue search
                pass
        parent = current.parent
        if parent == current:  # Reached filesystem root
            break
        current = parent

    # If no repo root found, try relative paths from cwd
    search_bases = [repo_root] if repo_root else [Path.cwd()]

    # Build candidate list based on preference
    candidates = []
    for base in search_bases:
        if prefer_release:
            # Release first, then debug
            candidates.extend(
                [
                    base / f"target/release/{binary_name}",
                    base / f"bridge/target/release/{binary_name}",
                    base / f"core/target/release/{binary_name}",
                    base / f"target/debug/{binary_name}",
                    base / f"bridge/target/debug/{binary_name}",
                    base / f"core/target/debug/{binary_name}",
                ]
            )
        else:
            # Debug first, then release
            candidates.extend(
                [
                    base / f"target/debug/{binary_name}",
                    base / f"bridge/target/debug/{binary_name}",
                    base / f"core/target/debug/{binary_name}",
                    base / f"target/release/{binary_name}",
                    base / f"bridge/target/release/{binary_name}",
                    base / f"core/target/release/{binary_name}",
                ]
            )

    for c in candidates:
        if c.exists() and c.is_file():
            return c.resolve()
    return None


def get_binary(
    binary_name: str,
    version: str = "latest",
    allow_local: bool = True,
    prefer_release: bool = True,
    force_download: bool = False,
) -> Path:
    """Get binary, downloading if necessary or using local build in dev mode.

    Priority order:
    1. Local build (target/release or target/debug) - if allow_local=True and not force_download
    2. Cached binary (~/.cache/loom/bin/{version}/{platform}/{binary_name}) - if valid
    3. Download from GitHub Releases

    Args:
        binary_name: Name of binary (e.g., "loom-core", "loom-bridge-server")
        version: Version to fetch (e.g., "latest", "0.1.0")
        allow_local: If True, will use local builds from cargo (default: True)
        prefer_release: If True, prefer release over debug builds (default: True)
        force_download: If True, skip cache and local builds, force download (default: False)

    Returns:
        Path to the binary (always in cache directory for consistency)

    Development workflow:
        # Build from source
        cd /path/to/loom
        cargo build --release

        # SDK will automatically detect and use new build
        loom up
    """
    cached = binary_path(binary_name, version)

    # Check local build first (highest priority for dev workflow)
    if allow_local and not force_download:
        local = find_local_build(binary_name, prefer_release=prefer_release)
        if local:
            local_version = get_binary_version(local)
            build_type = "release" if "release" in str(local) else "debug"

            print(f"[loom] Using local {build_type} build: {local}")
            if local_version:
                print(f"[loom]   Version: {local_version}")

            # Copy to cache for consistency
            cached.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(local, cached)
            ensure_executable(cached)
            return cached

    # Check cached binary with validation
    if not force_download and validate_cached_binary(cached, binary_name, version):
        cached_version = get_binary_version(cached)
        print(f"[loom] Using cached binary: {cached}")
        if cached_version:
            print(f"[loom]   Version: {cached_version}")
        return cached

    # Download from GitHub Releases
    print(f"[loom] Downloading {binary_name} version {version}...")
    return download_from_github(binary_name, version)


def start_binary(
    binary_name: str,
    env_vars: Optional[dict[str, str]] = None,
    version: str = "latest",
    allow_local: bool = True,
    prefer_release: bool = True,
    force_download: bool = False,
) -> subprocess.Popen:
    """Start a Loom runtime binary with given environment variables.

    Note: stdout and stderr are redirected to DEVNULL to prevent buffer filling
    and process hanging. For debugging, use direct binary execution or redirect
    to files in calling code.
    """
    binary = get_binary(
        binary_name,
        version=version,
        allow_local=allow_local,
        prefer_release=prefer_release,
        force_download=force_download,
    )
    env = os.environ.copy()
    if env_vars:
        env.update(env_vars)
    # Use DEVNULL to prevent pipe buffer filling and process hanging
    proc = subprocess.Popen(
        [str(binary)],
        env=env,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    return proc


# Convenience functions for specific binaries
def start_bridge(
    address: str,
    version: str = "latest",
    prefer_release: bool = True,
    force_download: bool = False,
) -> subprocess.Popen:
    """Start loom-bridge-server."""
    return start_binary(
        "loom-bridge-server",
        env_vars={"LOOM_BRIDGE_ADDR": address},
        version=version,
        prefer_release=prefer_release,
        force_download=force_download,
    )


def start_core(
    bridge_addr: str,
    dashboard_port: int = 3030,
    version: str = "latest",
    prefer_release: bool = True,
    force_download: bool = False,
) -> subprocess.Popen:
    """Start loom-core (full runtime with dashboard).

    Note: loom-core is embedded within loom-bridge-server, so this
    actually starts loom-bridge-server with Dashboard enabled.
    """
    return start_binary(
        "loom-bridge-server",
        env_vars={
            "LOOM_BRIDGE_ADDR": bridge_addr,
            "LOOM_DASHBOARD": "true",
            "LOOM_DASHBOARD_PORT": str(dashboard_port),
        },
        version=version,
        prefer_release=prefer_release,
        force_download=force_download,
    )
