"""Rich UI Components for Loom CLI.

This module provides beautiful terminal UI components using the Rich library.
All CLI visual elements should use these components for consistent styling.

Usage:
    from loom.cli.ui import console, LoomTheme, print_header, print_error

    # Direct console usage
    console.print("[bold cyan]Hello[/bold cyan] World!")

    # High-level components
    print_header()
    print_error("Something went wrong")
"""

from __future__ import annotations

from typing import Any, Optional

from rich.console import Console, Group
from rich.live import Live
from rich.markdown import Markdown
from rich.panel import Panel
from rich.progress import (
    BarColumn,
    Progress,
    SpinnerColumn,
    TaskProgressColumn,
    TextColumn,
    TimeElapsedColumn,
)
from rich.syntax import Syntax
from rich.table import Table
from rich.text import Text
from rich.theme import Theme

# ============================================================================
# Theme Configuration
# ============================================================================

LOOM_THEME = Theme(
    {
        # Base colors
        "info": "cyan",
        "warning": "yellow",
        "error": "bold red",
        "success": "bold green",
        # Agent/Chat specific
        "user": "bold blue",
        "assistant": "bold green",
        "system": "dim white",
        "thinking": "magenta",
        "tool": "cyan",
        "tool.name": "bold cyan",
        "tool.args": "dim",
        "tool.result": "green",
        "tool.error": "red",
        # Streaming
        "stream.text": "white",
        "stream.thinking": "dim magenta italic",
        "stream.code": "bright_white on grey23",
        # Status
        "status.pending": "yellow",
        "status.running": "cyan",
        "status.success": "green",
        "status.error": "red",
        # Header/Footer
        "header": "bold cyan",
        "header.title": "bold white on blue",
        "divider": "dim",
        # Stats
        "stat.label": "dim",
        "stat.value": "bold",
    }
)

# Global console instance with theme
console = Console(theme=LOOM_THEME)


# ============================================================================
# Header & Layout Components
# ============================================================================


def print_header(title: str = "Loom Chat", subtitle: str = "Interactive AI with Cognitive Loop"):
    """Print application header with branding."""
    header_text = Text()
    header_text.append("🧠 ", style="bold")
    header_text.append(title, style="bold white")

    panel = Panel(
        Group(
            Text(subtitle, style="dim", justify="center"),
        ),
        title=header_text,
        title_align="center",
        border_style="cyan",
        padding=(1, 2),
    )
    console.print(panel)


def print_divider(style: str = "dim", char: str = "─"):
    """Print a horizontal divider line."""
    console.rule(style=style, characters=char)


def print_welcome():
    """Print welcome message with quick help."""
    console.print()
    console.print("[dim]Type [yellow]/help[/yellow] for available commands[/dim]")
    console.print("[dim]Type [yellow]/quit[/yellow] to exit[/dim]")
    console.print()


# ============================================================================
# Message Components
# ============================================================================


def print_user_message(message: str):
    """Print user input message."""
    console.print()
    console.print(f"[user]You ▶[/user] {message}")


def print_assistant_message(message: str, show_as_markdown: bool = True):
    """Print assistant response message."""
    console.print()
    if show_as_markdown:
        md = Markdown(message)
        panel = Panel(
            md,
            title="[assistant]🤖 Assistant[/assistant]",
            title_align="left",
            border_style="green",
            padding=(1, 2),
        )
        console.print(panel)
    else:
        console.print(f"[assistant]🤖 Assistant:[/assistant] {message}")


def print_system_message(message: str):
    """Print system/info message."""
    console.print(f"[system]ℹ️  {message}[/system]")


def print_error(message: str, title: str = "Error"):
    """Print error message in a panel."""
    panel = Panel(
        f"[error]{message}[/error]",
        title=f"[error]❌ {title}[/error]",
        border_style="red",
    )
    console.print(panel)


def print_success(message: str):
    """Print success message."""
    console.print(f"[success]✅ {message}[/success]")


def print_warning(message: str):
    """Print warning message."""
    console.print(f"[warning]⚠️  {message}[/warning]")


# ============================================================================
# Thinking & Tool Components
# ============================================================================


def print_thinking_header():
    """Print thinking/streaming header."""
    console.print()
    console.print("[thinking]💭 Thinking...[/thinking]")
    console.rule(style="dim magenta", characters="·")


def print_thinking_step(
    step_num: int,
    reasoning: Optional[str] = None,
    tool_name: Optional[str] = None,
    tool_args: Optional[dict] = None,
    result: Optional[str] = None,
    error: Optional[str] = None,
    offloaded_path: Optional[str] = None,
):
    """Print a thinking step with tool call and result."""
    # Step header
    step_text = Text()
    step_text.append(f"Step {step_num}", style="bold magenta")

    content_parts = []

    # Reasoning
    if reasoning:
        content_parts.append(Text(f"💭 {reasoning}", style="stream.thinking"))

    # Tool call
    if tool_name:
        tool_text = Text()
        tool_text.append("🔧 Tool: ", style="tool")
        tool_text.append(tool_name, style="tool.name")
        if tool_args:
            args_str = str(tool_args)
            if len(args_str) > 80:
                args_str = args_str[:77] + "..."
            tool_text.append(f"\n   Args: {args_str}", style="tool.args")
        content_parts.append(tool_text)

    # Result/Error
    if error:
        content_parts.append(Text(f"❌ Error: {error}", style="tool.error"))
    elif offloaded_path:
        result_text = Text()
        result_text.append("✅ Result: ", style="tool.result")
        result_text.append(f"Offloaded to {offloaded_path}", style="dim")
        content_parts.append(result_text)
    elif result:
        # Truncate long results
        display_result = result
        if len(result) > 300:
            lines = result.split("\n")
            if len(lines) > 8:
                display_result = "\n".join(lines[:6]) + f"\n... ({len(lines) - 6} more lines)"
            else:
                display_result = result[:297] + "..."

        result_text = Text()
        result_text.append("✅ Result:\n", style="tool.result")
        result_text.append(display_result, style="dim")
        content_parts.append(result_text)

    # Combine and display
    content = Group(*content_parts) if content_parts else Text("(no content)")

    panel = Panel(
        content,
        title=step_text,
        title_align="left",
        border_style="magenta",
        padding=(0, 1),
    )
    console.print(panel)


def print_stats(
    iterations: int = 0,
    latency_ms: int = 0,
    tool_calls: int = 0,
    success: bool = True,
):
    """Print execution statistics."""
    stats = Table.grid(padding=(0, 2))
    stats.add_column(style="stat.label")
    stats.add_column(style="stat.value")

    if iterations > 0:
        stats.add_row("⚡ Iterations:", str(iterations))
    if latency_ms > 0:
        stats.add_row("⏱️  Latency:", f"{latency_ms}ms")
    if tool_calls > 0:
        stats.add_row("🔧 Tool calls:", str(tool_calls))

    status = "[success]✅ Success[/success]" if success else "[error]❌ Failed[/error]"
    stats.add_row("Status:", status)

    console.print(stats)


# ============================================================================
# Help & Commands
# ============================================================================


def print_help():
    """Print help message with available commands."""
    # Commands table
    commands = Table(
        title="[bold]Available Commands[/bold]",
        show_header=True,
        header_style="bold cyan",
        border_style="dim",
    )
    commands.add_column("Command", style="yellow")
    commands.add_column("Description")

    commands.add_row("/help", "Show this help message")
    commands.add_row("/clear", "Clear conversation history")
    commands.add_row("/history", "Show conversation history")
    commands.add_row("/verbose", "Toggle verbose mode (show thinking)")
    commands.add_row("/stream", "Toggle streaming mode")
    commands.add_row("/connect [agent]", "Connect to a specific agent")
    commands.add_row("/status", "Show connection status")
    commands.add_row("/quit", "Exit the chat")

    console.print()
    console.print(commands)

    # Tools table
    tools = Table(
        title="[bold]Available Tools[/bold]",
        show_header=True,
        header_style="bold cyan",
        border_style="dim",
    )
    tools.add_column("Tool", style="cyan")
    tools.add_column("Description")

    tools.add_row("weather:get", "Get weather for a location")
    tools.add_row("system:shell", "Run shell commands")
    tools.add_row("fs:read_file", "Read file contents")
    tools.add_row("fs:write_file", "Write content to a file")
    tools.add_row("fs:list_dir", "List directory contents")
    tools.add_row("web:search", "Search the web (Brave Search)")

    console.print()
    console.print(tools)
    console.print()


def print_history(history: list[dict]):
    """Print conversation history."""
    if not history:
        console.print("[dim]No conversation history yet.[/dim]")
        return

    console.print()
    console.print("[bold]📜 Conversation History[/bold]")

    for i, msg in enumerate(history, 1):
        role = msg.get("role", "unknown")
        content = msg.get("content", "")

        # Truncate long content
        if len(content) > 100:
            content = content[:97] + "..."

        if role == "user":
            console.print(f"  [dim][{i}][/dim] [user]You:[/user] {content}")
        else:
            console.print(f"  [dim][{i}][/dim] [assistant]AI:[/assistant] {content}")

    console.print()


# ============================================================================
# Permission Dialog
# ============================================================================


def print_permission_request(
    tool_name: str,
    args: dict,
    reason: str,
) -> bool:
    """Print permission request and get user input.

    Returns True if approved, False otherwise.
    """
    panel = Panel(
        Group(
            Text(f"Tool: {tool_name}", style="tool.name"),
            Text(f"Args: {args}", style="tool.args"),
            Text(f"Reason: {reason}", style="dim"),
        ),
        title="[warning]⚠️  Permission Required[/warning]",
        border_style="yellow",
    )
    console.print()
    console.print(panel)

    try:
        response = console.input("[yellow]Allow this action? [y/N]: [/yellow]").strip().lower()
        approved = response in ("y", "yes")

        if approved:
            console.print("[success]✅ Approved by user[/success]")
        else:
            console.print("[error]❌ Denied by user[/error]")

        return approved
    except (EOFError, KeyboardInterrupt):
        console.print("\n[error]❌ Denied (interrupted)[/error]")
        return False


# ============================================================================
# Progress & Status
# ============================================================================


def create_spinner(description: str = "Processing...") -> Progress:
    """Create a spinner progress bar for async operations."""
    return Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        console=console,
    )


def create_progress_bar(description: str = "Progress") -> Progress:
    """Create a progress bar for multi-step operations."""
    return Progress(
        SpinnerColumn(),
        TextColumn("[progress.description]{task.description}"),
        BarColumn(),
        TaskProgressColumn(),
        TimeElapsedColumn(),
        console=console,
    )


def print_connection_status(connected: bool, agent_id: str = "", bridge_addr: str = ""):
    """Print connection status."""
    status_table = Table.grid(padding=(0, 2))
    status_table.add_column()
    status_table.add_column()

    if connected:
        status_table.add_row("[success]●[/success]", "Connected")
        if agent_id:
            status_table.add_row("[dim]Agent:[/dim]", agent_id)
        if bridge_addr:
            status_table.add_row("[dim]Bridge:[/dim]", bridge_addr)
    else:
        status_table.add_row("[error]●[/error]", "Disconnected")

    panel = Panel(status_table, title="[bold]Connection Status[/bold]", border_style="dim")
    console.print(panel)


# ============================================================================
# Streaming Display
# ============================================================================


class StreamingDisplay:
    """Context manager for streaming text display using Rich Live."""

    def __init__(self, title: str = "Assistant"):
        self.title = title
        self.content: list[str] = []
        self.live: Optional[Live] = None
        self._thinking_shown = False

    def __enter__(self) -> "StreamingDisplay":
        self.live = Live(
            self._render(),
            console=console,
            refresh_per_second=10,
            transient=True,
        )
        self.live.__enter__()
        return self

    def __exit__(self, *args):
        if self.live:
            self.live.__exit__(*args)
            # Print final content
            self._print_final()

    def _render(self) -> Panel:
        """Render current content as a panel."""
        text = "".join(self.content) or "[dim]...[/dim]"
        return Panel(
            Markdown(text) if "\n" in text or len(text) > 100 else Text(text),
            title=f"[assistant]🤖 {self.title}[/assistant]",
            border_style="green",
            padding=(0, 1),
        )

    def _print_final(self):
        """Print the final content after Live closes."""
        if self.content:
            text = "".join(self.content)
            panel = Panel(
                Markdown(text),
                title=f"[assistant]🤖 {self.title}[/assistant]",
                border_style="green",
                padding=(1, 2),
            )
            console.print(panel)

    def append(self, chunk: str):
        """Append a chunk of text."""
        self.content.append(chunk)
        if self.live:
            self.live.update(self._render())

    def show_thinking(self, message: str = "Thinking..."):
        """Show thinking indicator."""
        if not self._thinking_shown:
            console.print(f"[thinking]💭 {message}[/thinking]")
            self._thinking_shown = True


# ============================================================================
# Code Display
# ============================================================================


def print_code(code: str, language: str = "python", title: Optional[str] = None):
    """Print syntax-highlighted code."""
    syntax = Syntax(code, language, theme="monokai", line_numbers=True)
    if title:
        panel = Panel(syntax, title=title, border_style="dim")
        console.print(panel)
    else:
        console.print(syntax)


def print_json(data: Any, title: Optional[str] = None):
    """Print JSON data with syntax highlighting."""
    import json

    json_str = json.dumps(data, indent=2, ensure_ascii=False)
    print_code(json_str, "json", title)
