"""Data Offloader for Context Engineering.

This module implements automatic offloading of large tool outputs to files,
keeping only references in context. This dramatically reduces token usage
for operations that return large amounts of data.

Offloading Philosophy:
1. File contents → write to .loom/cache/, return path
2. Search results → write to JSON, return summary + path
3. Large outputs → write to file, return first/last N lines + path
4. Small outputs → keep inline (no offloading)
"""

from __future__ import annotations

import hashlib
import json
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Optional, Union


@dataclass
class OffloadConfig:
    """Configuration for data offloading.

    Attributes:
        cache_dir: Directory for cached outputs (default: .loom/cache)
        size_threshold: Minimum size in bytes to trigger offload (default: 2048)
        line_threshold: Minimum lines to trigger offload (default: 50)
        preview_lines: Lines to keep in preview (default: 10)
        max_age_hours: Maximum age of cache files before cleanup (default: 24)
        enabled: Whether offloading is enabled (default: True)
    """

    cache_dir: str = ".loom/cache"
    size_threshold: int = 2048  # 2KB
    line_threshold: int = 50
    preview_lines: int = 10
    max_age_hours: int = 24
    enabled: bool = True


@dataclass
class OffloadResult:
    """Result of an offload operation.

    Attributes:
        offloaded: Whether data was offloaded
        content: The content to use (preview or original)
        file_path: Path to offloaded file (if offloaded)
        original_size: Original content size in bytes
        original_lines: Original line count
        content_hash: Hash of original content (for deduplication)
    """

    offloaded: bool
    content: str
    file_path: Optional[str] = None
    original_size: int = 0
    original_lines: int = 0
    content_hash: Optional[str] = None

    def to_observation(self, tool_name: str) -> str:
        """Generate observation string for Step.

        Args:
            tool_name: Name of the tool that produced this output

        Returns:
            Formatted observation string
        """
        if not self.offloaded:
            return self.content

        size_str = _format_size(self.original_size)
        return (
            f"Output ({self.original_lines} lines, {size_str}) "
            f"saved to {self.file_path}\n\n"
            f"Preview:\n{self.content}"
        )


class DataOffloader:
    """Offloads large data to files, keeping references in context.

    Usage:
        offloader = DataOffloader(workspace_path="/path/to/project")
        result = offloader.offload(
            content="...(large file content)...",
            category="file_read",
            identifier="config.json"
        )
        if result.offloaded:
            # Use result.content (preview) and result.file_path
        else:
            # Use result.content (original, was small enough)
    """

    def __init__(
        self,
        workspace_path: Union[str, Path],
        config: Optional[OffloadConfig] = None,
    ):
        """Initialize offloader.

        Args:
            workspace_path: Path to workspace root
            config: Offload configuration
        """
        self.workspace = Path(workspace_path)
        self.config = config or OffloadConfig()
        self._cache_dir: Optional[Path] = None

    @property
    def cache_dir(self) -> Path:
        """Get or create cache directory."""
        if self._cache_dir is None:
            self._cache_dir = self.workspace / self.config.cache_dir
            self._cache_dir.mkdir(parents=True, exist_ok=True)
        return self._cache_dir

    def offload(
        self,
        content: str,
        category: str,
        identifier: str,
        force: bool = False,
    ) -> OffloadResult:
        """Offload content if it exceeds thresholds.

        Args:
            content: Content to potentially offload
            category: Category (e.g., "file_read", "shell_output", "search")
            identifier: Unique identifier (e.g., filename, command)
            force: Force offload regardless of size

        Returns:
            OffloadResult with content or preview + path
        """
        if not self.config.enabled and not force:
            return OffloadResult(
                offloaded=False,
                content=content,
                original_size=len(content),
                original_lines=content.count("\n") + 1,
            )

        size = len(content)
        lines = content.count("\n") + 1
        content_hash = self._compute_hash(content)

        # Check thresholds
        should_offload = force or (
            size >= self.config.size_threshold or lines >= self.config.line_threshold
        )

        if not should_offload:
            return OffloadResult(
                offloaded=False,
                content=content,
                original_size=size,
                original_lines=lines,
                content_hash=content_hash,
            )

        # Check for existing cached file (deduplication)
        existing = self._find_cached(content_hash)
        if existing:
            preview = self._generate_preview(content, category)
            return OffloadResult(
                offloaded=True,
                content=preview,
                file_path=str(existing.relative_to(self.workspace)),
                original_size=size,
                original_lines=lines,
                content_hash=content_hash,
            )

        # Write to cache
        file_path = self._write_to_cache(content, category, identifier, content_hash)
        preview = self._generate_preview(content, category)

        return OffloadResult(
            offloaded=True,
            content=preview,
            file_path=str(file_path.relative_to(self.workspace)),
            original_size=size,
            original_lines=lines,
            content_hash=content_hash,
        )

    def offload_json(
        self,
        data: Any,
        category: str,
        identifier: str,
    ) -> OffloadResult:
        """Offload JSON-serializable data.

        Args:
            data: JSON-serializable data
            category: Category for the data
            identifier: Unique identifier

        Returns:
            OffloadResult
        """
        content = json.dumps(data, indent=2, ensure_ascii=False)
        return self.offload(content, category, identifier)

    def retrieve(self, file_path: str) -> Optional[str]:
        """Retrieve offloaded content by path.

        Args:
            file_path: Relative path to cached file

        Returns:
            Content if found, None otherwise
        """
        full_path = self.workspace / file_path
        if full_path.exists():
            return full_path.read_text(encoding="utf-8")
        return None

    def cleanup(self, max_age_hours: Optional[int] = None) -> int:
        """Remove old cached files.

        Args:
            max_age_hours: Max age in hours (uses config default if None)

        Returns:
            Number of files removed
        """
        max_age = max_age_hours or self.config.max_age_hours
        cutoff = time.time() - (max_age * 3600)
        removed = 0

        if not self.cache_dir.exists():
            return 0

        for file_path in self.cache_dir.rglob("*"):
            if file_path.is_file() and file_path.stat().st_mtime < cutoff:
                file_path.unlink()
                removed += 1

        return removed

    def _compute_hash(self, content: str) -> str:
        """Compute content hash for deduplication."""
        return hashlib.sha256(content.encode()).hexdigest()[:16]

    def _find_cached(self, content_hash: str) -> Optional[Path]:
        """Find existing cached file by hash."""
        pattern = f"*_{content_hash}.*"
        matches = list(self.cache_dir.glob(pattern))
        return matches[0] if matches else None

    def _write_to_cache(
        self,
        content: str,
        category: str,
        identifier: str,
        content_hash: str,
    ) -> Path:
        """Write content to cache file.

        Args:
            content: Content to write
            category: Category subdirectory
            identifier: Base filename
            content_hash: Content hash for dedup

        Returns:
            Path to written file
        """
        # Create category subdirectory
        category_dir = self.cache_dir / category
        category_dir.mkdir(parents=True, exist_ok=True)

        # Sanitize identifier for filename
        safe_id = self._sanitize_filename(identifier)

        # Determine extension
        ext = self._get_extension(category, content)

        filename = f"{safe_id}_{content_hash}{ext}"
        file_path = category_dir / filename

        file_path.write_text(content, encoding="utf-8")
        return file_path

    def _generate_preview(self, content: str, category: str) -> str:
        """Generate preview for offloaded content.

        Args:
            content: Full content
            category: Content category

        Returns:
            Preview string
        """
        lines = content.split("\n")
        n = self.config.preview_lines

        if len(lines) <= n * 2:
            # Small enough to show everything
            return content

        # Show first N and last N lines
        first = lines[:n]
        last = lines[-n:]
        omitted = len(lines) - n * 2

        preview_lines = first + [f"\n... ({omitted} lines omitted) ...\n"] + last
        return "\n".join(preview_lines)

    def _sanitize_filename(self, name: str) -> str:
        """Sanitize string for use as filename."""
        # Replace path separators and invalid chars
        safe = name.replace("/", "_").replace("\\", "_")
        safe = "".join(c for c in safe if c.isalnum() or c in "._-")
        # Limit length
        return safe[:50] if len(safe) > 50 else safe

    def _get_extension(self, category: str, content: str) -> str:
        """Determine file extension based on category and content."""
        if category in ("search", "json"):
            return ".json"
        elif category == "shell_output":
            return ".log"
        elif content.strip().startswith("{") or content.strip().startswith("["):
            return ".json"
        else:
            return ".txt"


def _format_size(size_bytes: int) -> str:
    """Format byte size to human-readable string."""
    if size_bytes < 1024:
        return f"{size_bytes}B"
    elif size_bytes < 1024 * 1024:
        return f"{size_bytes / 1024:.1f}KB"
    else:
        return f"{size_bytes / (1024 * 1024):.1f}MB"


__all__ = [
    "DataOffloader",
    "OffloadConfig",
    "OffloadResult",
]
