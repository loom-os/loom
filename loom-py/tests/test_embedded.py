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


@pytest.mark.skip(reason="Requires actual GitHub release")
def test_download_from_github():
    """Test downloading from GitHub releases (integration test)."""
    # This test is skipped by default as it requires:
    # 1. An actual release to exist
    # 2. Network access
    # 3. Release assets in the expected format
    pass
