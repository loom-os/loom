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


# ============================================================================
# ReAct Streaming Parser & Renderer
# ============================================================================


class ReactStreamParser:
    """Incrementally parse ReAct-style streaming output.

    Detects and extracts:
    - Thought: reasoning content
    - Action: tool call JSON
    - Observation: tool results (injected externally)
    - FINAL ANSWER: final response

    Usage:
        parser = ReactStreamParser()
        for chunk in stream:
            events = parser.feed(chunk)
            for event in events:
                if event["type"] == "thought":
                    render_thought(event["content"])
                elif event["type"] == "action":
                    render_action(event["tool"], event["args"])
                elif event["type"] == "final_answer":
                    render_final(event["content"])
    """

    def __init__(self):
        self.buffer = ""
        self.current_section: Optional[str] = None  # "thought", "action", "final"
        self.section_content = ""
        self._final_answer_seen = False
        self._action_complete = False

    def feed(self, chunk: str) -> list[dict]:
        """Feed a chunk of text and return parsed events.

        Args:
            chunk: Incoming text chunk from LLM stream

        Returns:
            List of events: [{"type": "thought"|"action"|"final_answer"|"text", ...}]
        """
        # If we already got final answer, ignore further content
        if self._final_answer_seen:
            return []

        events = []
        self.buffer += chunk

        while True:
            event = self._try_extract_section()
            if event:
                events.append(event)
                if event["type"] == "final_answer":
                    self._final_answer_seen = True
                    break
            else:
                break

        return events

    def _try_extract_section(self) -> Optional[dict]:
        """Try to extract a complete section from buffer."""
        import re

        # Check for FINAL ANSWER (highest priority)
        final_match = re.search(
            r"FINAL\s*ANSWER\s*:\s*",
            self.buffer,
            re.IGNORECASE,
        )
        if final_match:
            # Everything after "FINAL ANSWER:" is the answer
            content = self.buffer[final_match.end() :].strip()
            # Clean up any trailing markers that LLM might add
            content = re.split(r"\n(?:Thought|Action|FINAL ANSWER)\s*:?", content)[0].strip()
            self.buffer = ""
            if content:
                return {"type": "final_answer", "content": content}
            return None

        # Check for Thought section
        thought_match = re.search(
            r"(?:^|\n)Thought\s*(?:\d+)?\s*:\s*",
            self.buffer,
            re.IGNORECASE,
        )
        if thought_match:
            # Look for where thought ends (Action or another Thought or FINAL)
            rest = self.buffer[thought_match.end() :]
            end_match = re.search(
                r"\n(?:Action\s*:|Thought\s*\d*:|FINAL\s*ANSWER)",
                rest,
                re.IGNORECASE,
            )
            if end_match:
                thought_content = rest[: end_match.start()].strip()
                self.buffer = rest[end_match.start() :]
                if thought_content:
                    return {"type": "thought", "content": thought_content}
            # If no end marker yet, keep buffering
            return None

        # Check for Action section with JSON
        action_match = re.search(
            r"(?:^|\n)Action\s*:\s*",
            self.buffer,
            re.IGNORECASE,
        )
        if action_match:
            rest = self.buffer[action_match.end() :]
            # Try to find complete JSON
            json_result = self._extract_json(rest)
            if json_result:
                tool_name = (
                    json_result.get("tool") or json_result.get("action") or json_result.get("name")
                )
                args = (
                    json_result.get("args")
                    or json_result.get("arguments")
                    or json_result.get("input")
                    or {}
                )
                # Clear buffer up to after the JSON
                json_end = rest.find("}") + 1
                self.buffer = rest[json_end:].lstrip()
                if tool_name:
                    return {"type": "action", "tool": tool_name, "args": args}

            # Also try Python-style: tool_name({'arg': 'value'})
            python_match = re.match(
                r"([a-z_:]+)\s*\(\s*(\{.+?\})\s*\)",
                rest,
                re.IGNORECASE | re.DOTALL,
            )
            if python_match:
                tool_name = python_match.group(1)
                try:
                    import ast

                    args = ast.literal_eval(python_match.group(2))
                    self.buffer = rest[python_match.end() :].lstrip()
                    return {"type": "action", "tool": tool_name, "args": args}
                except (ValueError, SyntaxError):
                    pass

        # If no structured content, check if we have plain text to emit
        # Only emit if buffer is getting long and no markers found
        if len(self.buffer) > 200:
            # Check if there's any marker coming
            if not re.search(r"(?:Thought|Action|FINAL)", self.buffer, re.IGNORECASE):
                text = self.buffer[:100]
                self.buffer = self.buffer[100:]
                return {"type": "text", "content": text}

        return None

    def _extract_json(self, text: str) -> Optional[dict]:
        """Extract JSON object from text."""
        import json

        start = text.find("{")
        if start == -1:
            return None

        # Find matching closing brace
        depth = 0
        for i, char in enumerate(text[start:], start):
            if char == "{":
                depth += 1
            elif char == "}":
                depth -= 1
                if depth == 0:
                    try:
                        return json.loads(text[start : i + 1])
                    except json.JSONDecodeError:
                        return None
        return None

    def flush(self) -> list[dict]:
        """Flush remaining buffer content as events."""
        events = []
        if self.buffer.strip():
            # Check one more time for final answer
            import re

            final_match = re.search(
                r"FINAL\s*ANSWER\s*:\s*(.+)", self.buffer, re.IGNORECASE | re.DOTALL
            )
            if final_match:
                events.append({"type": "final_answer", "content": final_match.group(1).strip()})
            elif not self._final_answer_seen:
                # Emit as text
                events.append({"type": "text", "content": self.buffer.strip()})
        self.buffer = ""
        return events


class SimpleStreamRenderer:
    """Simple streaming renderer that appends output (no Live display).

    This renderer prints content as it arrives without using Rich Live,
    so output is never overwritten or cleared. Better for debugging
    and simpler user experience.

    Output format:
    ╭─ Step 1 ─────────────────────────────────────────────────────────────────────╮
    │ 💭 I need to check if the workspace directory exists...                       │
    │ 🔧 fs:list_dir                                                                │
    │    {"path": "workspace"}                                                      │
    │ ✅ Found 2 entries                                                            │
    ╰──────────────────────────────────────────────────────────────────────────────╯
    """

    def __init__(self, show_thinking: bool = True):
        self.show_thinking = show_thinking
        self.current_thought = ""
        self.current_action: Optional[dict] = None
        self.step_count = 0
        self.final_answer = ""
        self._in_step = False
        self._width = 80  # Fixed width for consistent box drawing

    def __enter__(self) -> "SimpleStreamRenderer":
        return self

    def __exit__(self, *args):
        # Close any open step
        self._close_step()
        # Print final answer if any
        if self.final_answer:
            self._print_final_answer()

    def _close_step(self):
        """Close the current step box if open."""
        if self._in_step:
            console.print(f"╰{'─' * (self._width - 2)}╯")
            self._in_step = False

    def _start_step(self):
        """Start a new step box."""
        self._close_step()
        self.step_count += 1
        header = f"─ Step {self.step_count} "
        padding = self._width - 2 - len(header)
        console.print(f"╭{header}{'─' * padding}╮")
        self._in_step = True

    def _print_line(self, content: str, prefix: str = ""):
        """Print a line within the step box."""
        # Calculate available width for content
        max_content = self._width - 4 - len(prefix)  # 4 = "│ " + " │" padding

        if len(content) > max_content:
            content = content[: max_content - 3] + "..."

        line = f"│ {prefix}{content}"
        padding = self._width - len(line) - 1
        console.print(f"{line}{' ' * max(0, padding)}│")

    def _print_final_answer(self):
        """Print the final answer in a clean format."""
        # Clean up the answer - remove any raw Thought/Action/FINAL ANSWER markers
        answer = self.final_answer

        # Extract just the actual final answer if it contains markers
        if "FINAL ANSWER:" in answer:
            parts = answer.split("FINAL ANSWER:")
            answer = parts[-1].strip()

        # Remove any trailing Thought/Action patterns
        lines = []
        skip_rest = False
        for line in answer.split("\n"):
            if skip_rest:
                continue
            if line.strip().startswith("Thought:") or line.strip().startswith("Action:"):
                continue
            lines.append(line)

        answer = "\n".join(lines).strip()

        if answer:
            console.print()
            print_assistant_message(answer)

    def pause_for_input(self):
        """No-op for simple renderer - no Live to pause."""

        class NoPause:
            def __enter__(self):
                return self

            def __exit__(self, *args):
                pass

        return NoPause()

    def feed(self, chunk: str) -> list[dict]:
        """Feed text chunk - for plain streaming text."""
        return []

    def flush(self) -> list[dict]:
        """Flush - close any open step."""
        self._close_step()
        return []

    def add_thought(self, thought: str):
        """Add a thinking step."""
        if not self.show_thinking:
            return

        self.current_thought = thought
        self._start_step()

        # Truncate thought for preview
        thought_preview = thought[:120] + "..." if len(thought) > 120 else thought
        thought_preview = thought_preview.replace("\n", " ")
        self._print_line(thought_preview, "💭 ")

    def add_action(self, tool_name: str, args: str):
        """Add a tool call action."""
        self.current_action = {"tool": tool_name, "args": args}

        if not self._in_step:
            self._start_step()

        self._print_line(tool_name, "🔧 ")

        # Format args nicely
        args_preview = args[:55] + "..." if len(args) > 55 else args
        self._print_line(args_preview, "   ")

    def add_observation(
        self, tool_name: str, result: str, success: bool = True, offloaded: Optional[str] = None
    ):
        """Add observation from tool execution."""
        if success:
            result_preview = result[:90] + "..." if len(result) > 90 else result
            result_preview = result_preview.replace("\n", " ")
            self._print_line(result_preview, "✅ ")
        else:
            error_preview = result[:90] if len(result) <= 90 else result[:87] + "..."
            self._print_line(error_preview, "❌ ")

        # Close step after observation
        self._close_step()

        self.current_action = None
        self.current_thought = ""

    def set_final_answer(self, answer: str):
        """Set the final answer."""
        self.final_answer = answer


class ReactStreamRenderer:
    """Render ReAct streaming output with Rich Live display.

    Provides a beautiful, structured display of:
    - 💭 Thought bubbles (collapsible reasoning)
    - 🔧 Action cards (tool calls with args)
    - 📤 Observation panels (tool results)
    - ✅ Final answer (highlighted response)

    Usage:
        async with ReactStreamRenderer() as renderer:
            async for chunk in llm_stream:
                renderer.feed(chunk)
            # Handle tool execution
            renderer.add_observation(tool_name, result)
    """

    def __init__(self, show_thinking: bool = True):
        self.show_thinking = show_thinking
        self.parser = ReactStreamParser()
        self.live: Optional[Live] = None

        # Current state for display
        self.current_thought = ""
        self.current_action: Optional[dict] = None
        self.steps: list[dict] = []  # Completed steps
        self.final_answer = ""
        self.streaming_text = ""  # For plain text streaming

    def _format_tool_result(self, tool_name: str, result: str) -> str:
        """Format tool result for better display.

        Handles special formatting for certain tools:
        - web:search, web:ddg, duckduckgo: Format search results nicely
        - weather:get: Format weather data
        """
        import json

        # Check for web search tools
        if any(name in tool_name.lower() for name in ["search", "ddg", "duckduckgo"]):
            try:
                data = json.loads(result)
                if isinstance(data, list):
                    # DuckDuckGo format: [{"title": ..., "body": ..., "href": ...}, ...]
                    if not data:
                        return "No results found"

                    lines = [f"Found {len(data)} results:"]
                    for r in data[:3]:  # Show top 3
                        title = r.get("title", "")
                        body = r.get("body", r.get("snippet", r.get("description", "")))
                        if body and len(body) > 150:
                            body = body[:147] + "..."
                        if title and body:
                            lines.append(f"• {title}\n  {body}")
                        elif title:
                            lines.append(f"• {title}")
                    if len(data) > 3:
                        lines.append(f"  ... and {len(data) - 3} more results")
                    return "\n".join(lines)
                elif isinstance(data, dict):
                    results = data.get("results", [])
                    if results:
                        return self._format_tool_result(tool_name, json.dumps(results))
            except (json.JSONDecodeError, TypeError):
                pass

        # Check for weather
        if "weather" in tool_name.lower():
            try:
                data = json.loads(result)
                if isinstance(data, dict):
                    # Format weather data
                    parts = []
                    if "temp" in data or "temperature" in data:
                        parts.append(f"🌡️ {data.get('temp', data.get('temperature'))}")
                    if "condition" in data:
                        parts.append(f"{data['condition']}")
                    if "humidity" in data:
                        parts.append(f"💧 {data['humidity']}")
                    if parts:
                        return " | ".join(parts)
            except (json.JSONDecodeError, TypeError):
                pass

        return result

    def __enter__(self) -> "ReactStreamRenderer":
        """Start Live display."""
        self.live = Live(
            self._render(),
            console=console,
            refresh_per_second=12,
            transient=True,  # Clear when done
        )
        self.live.__enter__()
        return self

    def __exit__(self, *args):
        """Stop Live display and print final content."""
        if self.live:
            self.live.__exit__(*args)
        self._print_final()

    def pause_for_input(self):
        """Temporarily stop Live display for user input.

        Use this before showing dialogs that require user interaction.
        Returns a context that will resume the display on exit.
        """

        class LivePauser:
            def __init__(self, renderer: "ReactStreamRenderer"):
                self.renderer = renderer
                self.was_running = False

            def __enter__(self):
                if self.renderer.live:
                    self.was_running = True
                    # Update one last time with current state
                    self.renderer.live.update(self.renderer._render())
                    self.renderer.live.stop()
                return self

            def __exit__(self, *args):
                if self.was_running and self.renderer.live:
                    self.renderer.live.start()

        return LivePauser(self)

    def feed(self, chunk: str) -> list[dict]:
        """Feed a chunk and update display.

        Returns parsed events for external handling (e.g., tool execution).
        """
        events = self.parser.feed(chunk)

        for event in events:
            self._handle_event(event)

        if self.live:
            self.live.update(self._render())

        return events

    def flush(self) -> list[dict]:
        """Flush parser and update display."""
        events = self.parser.flush()
        for event in events:
            self._handle_event(event)
        if self.live:
            self.live.update(self._render())
        return events

    def _handle_event(self, event: dict):
        """Handle a parsed event."""
        if event["type"] == "thought":
            # Save previous thought as a step if any
            if self.current_thought and not self.current_action:
                self.steps.append({"type": "thought", "content": self.current_thought})
            self.current_thought = event["content"]
            self.current_action = None

        elif event["type"] == "action":
            self.current_action = event
            # Don't add to steps yet - wait for observation

        elif event["type"] == "final_answer":
            self.final_answer = event["content"]

        elif event["type"] == "text":
            self.streaming_text += event["content"]

    def add_observation(
        self, tool_name: str, result: str, success: bool = True, offloaded: Optional[str] = None
    ):
        """Add observation from tool execution.

        Call this after executing a tool to complete the step.
        """
        # Format result for better display
        formatted_result = self._format_tool_result(tool_name, result)

        step = {
            "type": "step",
            "thought": self.current_thought,
            "action": self.current_action,
            "observation": {
                "tool": tool_name,
                "result": formatted_result,
                "success": success,
                "offloaded": offloaded,
            },
        }
        self.steps.append(step)
        self.current_thought = ""
        self.current_action = None

        if self.live:
            self.live.update(self._render())

    def _render(self) -> Group:
        """Render current state as Rich renderables."""
        renderables = []

        # Render completed steps (collapsed)
        for i, step in enumerate(self.steps, 1):
            if step["type"] == "thought":
                # Standalone thought (rare)
                if self.show_thinking:
                    thought_text = Text()
                    thought_text.append("💭 ", style="magenta")
                    thought_text.append(step["content"][:100], style="dim")
                    if len(step["content"]) > 100:
                        thought_text.append("...", style="dim")
                    renderables.append(thought_text)

            elif step["type"] == "step":
                # Complete thought + action + observation
                step_panel = self._render_step(i, step)
                renderables.append(step_panel)

        # Render current state (in progress)
        if self.current_thought and self.show_thinking:
            thinking_text = Text()
            thinking_text.append("💭 Thinking: ", style="bold magenta")
            thinking_text.append(self.current_thought, style="magenta italic")
            renderables.append(Panel(thinking_text, border_style="magenta", padding=(0, 1)))

        if self.current_action:
            action_text = Text()
            action_text.append("🔧 Calling: ", style="bold cyan")
            action_text.append(self.current_action["tool"], style="cyan bold")
            args_str = str(self.current_action.get("args", {}))
            if len(args_str) > 60:
                args_str = args_str[:57] + "..."
            action_text.append(f"\n   Args: {args_str}", style="dim")
            action_text.append("\n   ⏳ Executing...", style="yellow")
            renderables.append(Panel(action_text, border_style="cyan", padding=(0, 1)))

        # Streaming text (for non-ReAct content)
        if self.streaming_text:
            renderables.append(Text(self.streaming_text, style="white"))

        # Final answer preview
        if self.final_answer:
            answer_text = Text()
            answer_text.append("✅ ", style="green")
            answer_text.append(self.final_answer[:200], style="white")
            if len(self.final_answer) > 200:
                answer_text.append("...", style="dim")
            renderables.append(
                Panel(answer_text, border_style="green", title="[green]Answer[/green]")
            )

        if not renderables:
            renderables.append(Text("💭 Thinking...", style="dim magenta"))

        return Group(*renderables)

    def _render_step(self, step_num: int, step: dict) -> Panel:
        """Render a complete step as a panel."""
        content_parts = []

        # Thought
        if step.get("thought") and self.show_thinking:
            thought_text = Text()
            thought_text.append("💭 ", style="magenta")
            thought = step["thought"]
            if len(thought) > 150:
                thought = thought[:147] + "..."
            thought_text.append(thought, style="dim italic")
            content_parts.append(thought_text)

        # Action
        if step.get("action"):
            action = step["action"]
            action_text = Text()
            action_text.append("🔧 ", style="cyan")
            action_text.append(action["tool"], style="bold cyan")
            content_parts.append(action_text)

        # Observation
        if step.get("observation"):
            obs = step["observation"]
            obs_text = Text()
            if obs["success"]:
                obs_text.append("✅ ", style="green")
                if obs.get("offloaded"):
                    obs_text.append(f"→ {obs['offloaded']}", style="dim")
                else:
                    result = obs["result"]
                    if len(result) > 100:
                        result = result[:97] + "..."
                    obs_text.append(result, style="dim")
            else:
                obs_text.append("❌ ", style="red")
                obs_text.append(obs["result"][:100], style="red dim")
            content_parts.append(obs_text)

        content = Group(*content_parts) if content_parts else Text("...")

        return Panel(
            content,
            title=f"[dim]Step {step_num}[/dim]",
            title_align="left",
            border_style="dim",
            padding=(0, 1),
        )

    def _print_final(self):
        """Print final formatted output after Live closes."""
        # Print completed steps summary
        if self.steps and self.show_thinking:
            console.print()
            for i, step in enumerate(self.steps, 1):
                if step["type"] == "step":
                    print_thinking_step(
                        step_num=i,
                        reasoning=step.get("thought"),
                        tool_name=step["action"]["tool"] if step.get("action") else None,
                        tool_args=step["action"].get("args") if step.get("action") else None,
                        result=(
                            step["observation"]["result"]
                            if step.get("observation", {}).get("success")
                            else None
                        ),
                        error=(
                            step["observation"]["result"]
                            if step.get("observation") and not step["observation"]["success"]
                            else None
                        ),
                        offloaded_path=step.get("observation", {}).get("offloaded"),
                    )

        # Print final answer
        if self.final_answer:
            print_assistant_message(self.final_answer)
        elif self.streaming_text:
            print_assistant_message(self.streaming_text)


# ============================================================================
# Exports
# ============================================================================
