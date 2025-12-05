"""Tests for DataOffloader."""

import json
import tempfile
from pathlib import Path

import pytest

from loom.context.offloader import (
    DataOffloader,
    OffloadConfig,
    OffloadResult,
)


class TestOffloadConfig:
    """Tests for OffloadConfig."""

    def test_default_values(self):
        """Test default configuration values."""
        config = OffloadConfig()
        assert config.cache_dir == ".loom/cache"
        assert config.size_threshold == 2048
        assert config.line_threshold == 50
        assert config.preview_lines == 10
        assert config.max_age_hours == 24
        assert config.enabled is True

    def test_custom_values(self):
        """Test custom configuration."""
        config = OffloadConfig(
            cache_dir=".cache/loom",
            size_threshold=4096,
            line_threshold=100,
            enabled=False,
        )
        assert config.cache_dir == ".cache/loom"
        assert config.size_threshold == 4096
        assert config.line_threshold == 100
        assert config.enabled is False


class TestOffloadResult:
    """Tests for OffloadResult."""

    def test_not_offloaded(self):
        """Test result when content was not offloaded."""
        result = OffloadResult(
            offloaded=False,
            content="small content",
            original_size=13,
            original_lines=1,
        )
        assert not result.offloaded
        assert result.file_path is None
        assert result.content == "small content"

    def test_offloaded(self):
        """Test result when content was offloaded."""
        result = OffloadResult(
            offloaded=True,
            content="first line\n...\nlast line",
            file_path=".loom/cache/file_read/test_abc123.txt",
            original_size=5000,
            original_lines=100,
            content_hash="abc123def456",
        )
        assert result.offloaded
        assert result.file_path is not None
        assert "abc123" in result.file_path

    def test_to_observation_not_offloaded(self):
        """Test observation for non-offloaded content."""
        result = OffloadResult(
            offloaded=False,
            content="Hello World",
            original_size=11,
            original_lines=1,
        )
        obs = result.to_observation("test_tool")
        assert obs == "Hello World"

    def test_to_observation_offloaded(self):
        """Test observation for offloaded content."""
        result = OffloadResult(
            offloaded=True,
            content="Preview content...",
            file_path=".loom/cache/test.txt",
            original_size=5000,
            original_lines=100,
        )
        obs = result.to_observation("fs:read_file")
        assert "100 lines" in obs
        assert ".loom/cache/test.txt" in obs
        assert "Preview content" in obs


class TestDataOffloader:
    """Tests for DataOffloader."""

    @pytest.fixture
    def temp_workspace(self):
        """Create a temporary workspace directory."""
        with tempfile.TemporaryDirectory() as tmpdir:
            yield Path(tmpdir)

    def test_small_content_not_offloaded(self, temp_workspace):
        """Test that small content is not offloaded."""
        offloader = DataOffloader(temp_workspace)
        content = "small content"

        result = offloader.offload(content, "test", "small.txt")

        assert not result.offloaded
        assert result.content == content
        assert result.file_path is None

    def test_large_content_offloaded_by_size(self, temp_workspace):
        """Test that large content is offloaded by size threshold."""
        config = OffloadConfig(size_threshold=100, line_threshold=1000)
        offloader = DataOffloader(temp_workspace, config)

        content = "x" * 200  # 200 bytes, exceeds 100 threshold

        result = offloader.offload(content, "test", "large.txt")

        assert result.offloaded
        assert result.file_path is not None
        assert result.original_size == 200

        # Verify file was written
        full_path = temp_workspace / result.file_path
        assert full_path.exists()
        assert full_path.read_text() == content

    def test_large_content_offloaded_by_lines(self, temp_workspace):
        """Test that content with many lines is offloaded."""
        config = OffloadConfig(size_threshold=100000, line_threshold=10)
        offloader = DataOffloader(temp_workspace, config)

        content = "\n".join([f"line {i}" for i in range(20)])  # 20 lines

        result = offloader.offload(content, "file_read", "many_lines.txt")

        assert result.offloaded
        assert result.original_lines == 20

    def test_preview_generation(self, temp_workspace):
        """Test that preview shows first and last lines."""
        config = OffloadConfig(
            size_threshold=100,
            line_threshold=5,
            preview_lines=2,
        )
        offloader = DataOffloader(temp_workspace, config)

        lines = [f"line {i}" for i in range(20)]
        content = "\n".join(lines)

        result = offloader.offload(content, "file_read", "preview_test.txt")

        assert result.offloaded
        # Preview should have first 2 and last 2 lines
        assert "line 0" in result.content
        assert "line 1" in result.content
        assert "line 18" in result.content
        assert "line 19" in result.content
        assert "omitted" in result.content

    def test_deduplication(self, temp_workspace):
        """Test that identical content has same hash (file may differ by name)."""
        config = OffloadConfig(size_threshold=10)
        offloader = DataOffloader(temp_workspace, config)

        content = "x" * 100  # Same content

        result1 = offloader.offload(content, "test", "file1.txt")
        result2 = offloader.offload(content, "test", "file2.txt")

        # Both should have same content hash
        assert result1.content_hash == result2.content_hash

        # Second call should find existing cached file
        # The implementation looks for files with matching hash
        # Both results point to files with the hash in the name
        assert result1.content_hash in result1.file_path
        assert result2.content_hash in result2.file_path

    def test_different_content_not_deduplicated(self, temp_workspace):
        """Test that different content creates different files."""
        config = OffloadConfig(size_threshold=10)
        offloader = DataOffloader(temp_workspace, config)

        result1 = offloader.offload("content A" * 20, "test", "file1.txt")
        result2 = offloader.offload("content B" * 20, "test", "file2.txt")

        assert result1.file_path != result2.file_path
        assert result1.content_hash != result2.content_hash

    def test_retrieve_content(self, temp_workspace):
        """Test retrieving offloaded content."""
        config = OffloadConfig(size_threshold=10)
        offloader = DataOffloader(temp_workspace, config)

        original = "x" * 100
        result = offloader.offload(original, "test", "retrieve.txt")

        retrieved = offloader.retrieve(result.file_path)
        assert retrieved == original

    def test_retrieve_nonexistent(self, temp_workspace):
        """Test retrieving non-existent file returns None."""
        offloader = DataOffloader(temp_workspace)

        result = offloader.retrieve(".loom/cache/nonexistent.txt")
        assert result is None

    def test_force_offload(self, temp_workspace):
        """Test force offload bypasses thresholds."""
        offloader = DataOffloader(temp_workspace)

        small_content = "tiny"
        result = offloader.offload(small_content, "test", "force.txt", force=True)

        assert result.offloaded
        assert result.file_path is not None

    def test_disabled_offloading(self, temp_workspace):
        """Test that disabled offloading never offloads."""
        config = OffloadConfig(enabled=False, size_threshold=10)
        offloader = DataOffloader(temp_workspace, config)

        large_content = "x" * 1000
        result = offloader.offload(large_content, "test", "disabled.txt")

        assert not result.offloaded
        assert result.content == large_content

    def test_json_offload(self, temp_workspace):
        """Test offloading JSON data."""
        config = OffloadConfig(size_threshold=10)
        offloader = DataOffloader(temp_workspace, config)

        data = {"items": [{"id": i, "value": f"item_{i}"} for i in range(50)]}
        result = offloader.offload_json(data, "search", "results.json")

        assert result.offloaded
        assert ".json" in result.file_path

        # Verify JSON can be read back
        retrieved = offloader.retrieve(result.file_path)
        parsed = json.loads(retrieved)
        assert parsed == data

    def test_cache_directory_creation(self, temp_workspace):
        """Test cache directory is created on demand."""
        config = OffloadConfig(cache_dir="custom/cache/path", size_threshold=10)
        offloader = DataOffloader(temp_workspace, config)

        # Cache dir shouldn't exist yet
        cache_path = temp_workspace / "custom/cache/path"
        assert not cache_path.exists()

        # Offload something
        offloader.offload("x" * 100, "test", "trigger.txt")

        # Now it should exist
        assert cache_path.exists()

    def test_cleanup_old_files(self, temp_workspace):
        """Test cleanup removes old files."""
        import os
        import time

        config = OffloadConfig(size_threshold=10, max_age_hours=1)
        offloader = DataOffloader(temp_workspace, config)

        # Create an old file
        result = offloader.offload("x" * 100, "test", "old.txt")
        old_file = temp_workspace / result.file_path

        # Make it appear old (modify mtime)
        old_time = time.time() - (2 * 3600)  # 2 hours ago
        os.utime(old_file, (old_time, old_time))

        # Create a new file
        offloader.offload("y" * 100, "test", "new.txt")

        # Cleanup with 1 hour max age
        removed = offloader.cleanup(max_age_hours=1)

        assert removed == 1
        assert not old_file.exists()

    def test_filename_sanitization(self, temp_workspace):
        """Test that filenames are sanitized."""
        config = OffloadConfig(size_threshold=10)
        offloader = DataOffloader(temp_workspace, config)

        # Use path-like identifier
        result = offloader.offload("x" * 100, "file_read", "/home/user/path/to/file.txt")

        assert result.offloaded
        # Filename should not contain path separators
        assert "/" not in Path(result.file_path).name
        assert "\\" not in Path(result.file_path).name

    def test_extension_detection_json(self, temp_workspace):
        """Test JSON extension detection."""
        config = OffloadConfig(size_threshold=10)
        offloader = DataOffloader(temp_workspace, config)

        json_content = '{"key": "value"}'
        result = offloader.offload(json_content * 10, "test", "auto.txt")

        assert ".json" in result.file_path

    def test_extension_for_category(self, temp_workspace):
        """Test extension based on category."""
        config = OffloadConfig(size_threshold=10)
        offloader = DataOffloader(temp_workspace, config)

        result = offloader.offload("output" * 50, "shell_output", "cmd")
        assert ".log" in result.file_path


class TestIntegration:
    """Integration tests for offloader with reducer."""

    @pytest.fixture
    def temp_workspace(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            yield Path(tmpdir)

    def test_offloader_with_step_reducer(self, temp_workspace):
        """Test using offloader alongside step reducer."""
        from loom.context.reducer import StepReducer

        offloader = DataOffloader(temp_workspace, OffloadConfig(size_threshold=100))
        reducer = StepReducer()

        # Simulate reading a large file
        large_content = "\n".join([f"line {i}: {'x' * 50}" for i in range(100)])

        # Offload the content
        offload_result = offloader.offload(large_content, "file_read", "large_file.py")

        # Reduce the step
        step = reducer.reduce(
            tool_name="fs:read_file",
            args={"path": "/project/large_file.py"},
            result=offload_result.content if offload_result.offloaded else large_content,
            success=True,
        )

        # Step should have reduced observation
        assert "large_file.py" in step.observation
        assert step.success

        # If offloaded, we can store the reference
        if offload_result.offloaded:
            step.outcome_ref = offload_result.file_path
            assert step.outcome_ref is not None
