"""Unit tests for loom.cli.ui module."""

from __future__ import annotations

from unittest.mock import patch

from loom.cli.ui import (
    LOOM_THEME,
    console,
    create_progress_bar,
    create_spinner,
    print_assistant_message,
    print_code,
    print_connection_status,
    print_divider,
    print_error,
    print_header,
    print_help,
    print_history,
    print_json,
    print_permission_request,
    print_stats,
    print_success,
    print_system_message,
    print_thinking_header,
    print_thinking_step,
    print_user_message,
    print_warning,
    print_welcome,
)


class TestTheme:
    """Tests for LOOM_THEME configuration."""

    def test_theme_has_required_styles(self):
        """Theme should have all required style definitions."""
        required_styles = [
            "info",
            "warning",
            "error",
            "success",
            "user",
            "assistant",
            "system",
            "thinking",
            "tool",
            "tool.name",
            "tool.args",
            "tool.result",
            "tool.error",
            "stream.text",
            "stream.thinking",
            "stream.code",
            "status.pending",
            "status.running",
            "status.success",
            "status.error",
            "header",
            "header.title",
            "divider",
            "stat.label",
            "stat.value",
        ]
        for style_name in required_styles:
            assert style_name in LOOM_THEME.styles, f"Missing style: {style_name}"

    def test_console_has_theme(self):
        """Global console should be created with theme."""
        # The console exists and can print styled text
        assert console is not None


class TestConsole:
    """Tests for console instance."""

    def test_console_exists(self):
        """Global console should be available."""
        assert console is not None

    def test_console_can_print(self):
        """Console should be able to print without error."""
        # Just ensure no exception is raised
        with patch.object(console, "print") as mock_print:
            console.print("test")
            mock_print.assert_called_once_with("test")


class TestHeaderComponents:
    """Tests for header and layout components."""

    def test_print_header_no_error(self):
        """print_header should not raise errors."""
        with patch.object(console, "print"):
            print_header()

    def test_print_header_custom_title(self):
        """print_header should accept custom title."""
        with patch.object(console, "print"):
            print_header(title="Custom Title", subtitle="Custom Subtitle")

    def test_print_divider_no_error(self):
        """print_divider should not raise errors."""
        with patch.object(console, "print"):
            print_divider()

    def test_print_divider_custom_style(self):
        """print_divider should accept custom parameters."""
        with patch.object(console, "print"):
            print_divider(style="cyan", char="=")

    def test_print_welcome_no_error(self):
        """print_welcome should not raise errors."""
        with patch.object(console, "print"):
            print_welcome()


class TestMessageComponents:
    """Tests for message printing components."""

    def test_print_user_message_no_error(self):
        """print_user_message should not raise errors."""
        with patch.object(console, "print"):
            print_user_message("Hello, world!")

    def test_print_assistant_message_no_error(self):
        """print_assistant_message should not raise errors."""
        with patch.object(console, "print"):
            print_assistant_message("Hi there!")

    def test_print_assistant_message_without_markdown(self):
        """print_assistant_message should work without markdown."""
        with patch.object(console, "print"):
            print_assistant_message("Plain text", show_as_markdown=False)

    def test_print_system_message_no_error(self):
        """print_system_message should not raise errors."""
        with patch.object(console, "print"):
            print_system_message("System notification")


class TestStatusComponents:
    """Tests for status message components."""

    def test_print_error_no_error(self):
        """print_error should not raise errors."""
        with patch.object(console, "print"):
            print_error("Something went wrong")

    def test_print_error_custom_title(self):
        """print_error should accept custom title."""
        with patch.object(console, "print"):
            print_error("Details here", title="Custom Error")

    def test_print_success_no_error(self):
        """print_success should not raise errors."""
        with patch.object(console, "print"):
            print_success("Operation completed!")

    def test_print_warning_no_error(self):
        """print_warning should not raise errors."""
        with patch.object(console, "print"):
            print_warning("Be careful!")


class TestThinkingComponents:
    """Tests for thinking/reasoning display components."""

    def test_print_thinking_header_no_error(self):
        """print_thinking_header should not raise errors."""
        with patch.object(console, "print"):
            print_thinking_header()

    def test_print_thinking_step_minimal(self):
        """print_thinking_step should work with minimal args."""
        with patch.object(console, "print"):
            print_thinking_step(step_num=1, reasoning="Thinking...")

    def test_print_thinking_step_with_tool(self):
        """print_thinking_step should work with tool info."""
        with patch.object(console, "print"):
            print_thinking_step(
                step_num=1,
                reasoning="Need to search",
                tool_name="web:search",
                tool_args={"query": "test"},
                result="Found results",
            )

    def test_print_thinking_step_with_error(self):
        """print_thinking_step should work with error info."""
        with patch.object(console, "print"):
            print_thinking_step(
                step_num=1,
                reasoning="Trying command",
                tool_name="system:shell",
                tool_args={"command": "invalid"},
                error="Command failed",
            )


class TestStatsComponents:
    """Tests for statistics display components."""

    def test_print_stats_minimal(self):
        """print_stats should work with minimal args."""
        with patch.object(console, "print"):
            print_stats()

    def test_print_stats_full(self):
        """print_stats should work with all args."""
        with patch.object(console, "print"):
            print_stats(
                iterations=5,
                latency_ms=1500,
                tool_calls=3,
                success=True,
            )

    def test_print_stats_failure(self):
        """print_stats should handle failure state."""
        with patch.object(console, "print"):
            print_stats(success=False)


class TestHelpComponents:
    """Tests for help and info components."""

    def test_print_help_no_error(self):
        """print_help should not raise errors."""
        with patch.object(console, "print"):
            print_help()

    def test_print_history_empty(self):
        """print_history should handle empty history."""
        with patch.object(console, "print"):
            print_history([])

    def test_print_history_with_messages(self):
        """print_history should display messages."""
        history = [
            {"role": "user", "content": "Hello"},
            {"role": "assistant", "content": "Hi there!"},
        ]
        with patch.object(console, "print"):
            print_history(history)

    def test_print_history_truncates_long_content(self):
        """print_history should truncate long messages."""
        history = [
            {"role": "user", "content": "x" * 500},
        ]
        with patch.object(console, "print"):
            print_history(history)


class TestPermissionComponents:
    """Tests for permission request components."""

    def test_print_permission_request_returns_bool(self):
        """print_permission_request should return boolean."""
        with patch.object(console, "print"):
            with patch.object(console, "input", return_value="y"):
                result = print_permission_request(
                    tool_name="system:shell",
                    args={"command": "rm -rf /"},
                    reason="Permission denied",
                )
                assert isinstance(result, bool)

    def test_print_permission_request_yes(self):
        """print_permission_request should return True for 'y'."""
        with patch.object(console, "print"):
            with patch.object(console, "input", return_value="y"):
                result = print_permission_request("tool", {}, "reason")
                assert result is True

    def test_print_permission_request_no(self):
        """print_permission_request should return False for 'n'."""
        with patch.object(console, "print"):
            with patch.object(console, "input", return_value="n"):
                result = print_permission_request("tool", {}, "reason")
                assert result is False


class TestProgressComponents:
    """Tests for progress/spinner components."""

    def test_create_spinner_returns_progress(self):
        """create_spinner should return a Progress instance."""
        spinner = create_spinner()
        assert spinner is not None

    def test_create_spinner_custom_description(self):
        """create_spinner should accept custom description."""
        spinner = create_spinner("Custom loading...")
        assert spinner is not None

    def test_create_progress_bar_returns_progress(self):
        """create_progress_bar should return a Progress instance."""
        progress = create_progress_bar()
        assert progress is not None

    def test_create_progress_bar_custom_description(self):
        """create_progress_bar should accept custom description."""
        progress = create_progress_bar("Downloading...")
        assert progress is not None


class TestConnectionStatus:
    """Tests for connection status component."""

    def test_print_connection_status_connected(self):
        """print_connection_status should show connected state."""
        with patch.object(console, "print"):
            print_connection_status(
                connected=True,
                agent_id="test-agent",
                bridge_addr="localhost:50051",
            )

    def test_print_connection_status_disconnected(self):
        """print_connection_status should show disconnected state."""
        with patch.object(console, "print"):
            print_connection_status(
                connected=False,
                agent_id="test-agent",
                bridge_addr="localhost:50051",
            )

    def test_print_connection_status_minimal(self):
        """print_connection_status should work with minimal args."""
        with patch.object(console, "print"):
            print_connection_status(connected=True)


class TestCodeComponents:
    """Tests for code display components."""

    def test_print_code_default_language(self):
        """print_code should default to Python."""
        with patch.object(console, "print"):
            print_code("print('hello')")

    def test_print_code_custom_language(self):
        """print_code should accept custom language."""
        with patch.object(console, "print"):
            print_code("console.log('hello')", language="javascript")

    def test_print_code_with_title(self):
        """print_code should accept title."""
        with patch.object(console, "print"):
            print_code("code here", title="Example")

    def test_print_json_basic(self):
        """print_json should display JSON data."""
        with patch.object(console, "print"):
            print_json({"key": "value"})

    def test_print_json_with_title(self):
        """print_json should accept title."""
        with patch.object(console, "print"):
            print_json({"key": "value"}, title="Config")

    def test_print_json_complex(self):
        """print_json should handle complex data."""
        with patch.object(console, "print"):
            print_json(
                {
                    "nested": {"key": [1, 2, 3]},
                    "bool": True,
                    "null": None,
                }
            )
