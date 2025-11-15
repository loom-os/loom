"""Tests for embedded runtime management."""

from pathlib import Path

import pytest

from loom.embedded import (
    binary_path,
    cache_dir,
    find_local_build,
    platform_tag,
    verify_checksum,
)


def test_platform_tag():
    """Test platform tag generation."""
    tag = platform_tag()
    assert isinstance(tag, str)
    assert "-" in tag
    # Should be format: {os}-{arch}
    parts = tag.split("-")
    assert len(parts) == 2
    assert parts[0] in ("linux", "macos", "windows", "darwin")


def test_cache_dir():
    """Test cache directory path."""
    cache = cache_dir()
    assert isinstance(cache, Path)
    assert "loom" in str(cache).lower()


def test_binary_path():
    """Test binary path generation."""
    path = binary_path("test-binary", version="0.1.0")
    assert isinstance(path, Path)
    assert "test-binary" in str(path)
    assert "0.1.0" in str(path)
    assert platform_tag() in str(path)


def test_verify_checksum(tmp_path):
    """Test checksum verification."""
    test_file = tmp_path / "test.bin"
    test_content = b"test content for checksum"
    test_file.write_bytes(test_content)

    # Compute expected checksum
    import hashlib

    expected = hashlib.sha256(test_content).hexdigest()

    # Should pass with correct checksum
    assert verify_checksum(test_file, expected)

    # Should fail with wrong checksum
    assert not verify_checksum(test_file, "0" * 64)

    # Should pass if no checksum provided
    assert verify_checksum(test_file, None)


def test_find_local_build():
    """Test finding local build (will be None in CI)."""
    result = find_local_build("loom-bridge-server")
    # In CI, this will be None; in dev repo, might find a build
    assert result is None or isinstance(result, Path)


def test_find_local_build_with_mock_repo(tmp_path, monkeypatch):
    """Test find_local_build with a mock repository structure."""
    # Create a mock Loom repository structure
    repo_root = tmp_path / "loom"
    repo_root.mkdir()

    # Create Cargo.toml with workspace marker
    (repo_root / "Cargo.toml").write_text(
        """
[workspace]
members = ["core", "bridge"]
"""
    )

    # Create target/release directory with binary
    target_release = repo_root / "target" / "release"
    target_release.mkdir(parents=True)
    binary = target_release / "loom-bridge-server"
    binary.write_text("mock binary")

    # Change to a nested directory within the repo
    nested_dir = repo_root / "demo" / "market-analyst"
    nested_dir.mkdir(parents=True)
    monkeypatch.chdir(nested_dir)

    # Should find the binary by walking up to repo root
    result = find_local_build("loom-bridge-server")
    assert result is not None
    assert result.name == "loom-bridge-server"
    # Check path components instead of string to be cross-platform
    assert result.parent.name == "release"
    assert result.parent.parent.name == "target"


def test_find_local_build_no_repo(tmp_path, monkeypatch):
    """Test find_local_build when not in a repo."""
    # Change to a directory without Cargo.toml
    monkeypatch.chdir(tmp_path)

    # Should return None
    result = find_local_build("loom-bridge-server")
    assert result is None


def test_find_local_build_prefers_release(tmp_path, monkeypatch):
    """Test that find_local_build prefers release over debug builds."""
    repo_root = tmp_path / "loom"
    repo_root.mkdir()

    (repo_root / "Cargo.toml").write_text("[workspace]\nmembers = []")

    # Create both debug and release binaries
    debug_dir = repo_root / "target" / "debug"
    debug_dir.mkdir(parents=True)
    (debug_dir / "loom-bridge-server").write_text("debug binary")

    release_dir = repo_root / "target" / "release"
    release_dir.mkdir(parents=True)
    (release_dir / "loom-bridge-server").write_text("release binary")

    monkeypatch.chdir(repo_root)

    # Should find release build first (it's listed first in candidates)
    result = find_local_build("loom-bridge-server")
    assert result is not None
    assert "release" in str(result)


@pytest.mark.skip(reason="Requires actual GitHub release")
def test_download_from_github():
    """Test downloading from GitHub releases (integration test)."""
    # This test is skipped by default as it requires:
    # 1. An actual release to exist
    # 2. Network access
    # 3. Release assets in the expected format
    pass
