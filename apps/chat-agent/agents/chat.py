#!/usr/bin/env python3
"""Chat Agent - Interactive cognitive agent with tool use.

This agent provides an interactive chat interface with:
- ReAct reasoning pattern (Thought -> Action -> Observation)
- Tool calling (weather, shell, file reading)
- Multi-turn conversation with memory
- Real-time display of thinking process

Run with: loom run
Or directly: python agents/chat.py
"""

import asyncio
import os
import sys
import shutil
from pathlib import Path
from datetime import datetime

# Load .env from parent directory
def _load_dotenv():
    for env_path in [Path(__file__).parent.parent / ".env", Path(".env")]:
        if env_path.exists():
            for line in env_path.read_text().splitlines():
                line = line.strip()
                if line and not line.startswith('#') and '=' in line:
                    key, _, value = line.partition('=')
                    key = key.strip()
                    value = value.strip().strip("'\"")
                    if key and key not in os.environ:
                        os.environ[key] = value
            break

_load_dotenv()

# Add loom-py to path for local development
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent / "loom-py" / "src"))

from loom import Agent, CognitiveAgent, CognitiveConfig, ThinkingStrategy
from loom.config import load_project_config
from loom.llm import LLMProvider


# ============================================================================
# Terminal UI Helpers
# ============================================================================

class Colors:
    """ANSI color codes for terminal output."""
    RESET = "\033[0m"
    BOLD = "\033[1m"
    DIM = "\033[2m"
    ITALIC = "\033[3m"
    UNDERLINE = "\033[4m"

    # Colors
    RED = "\033[31m"
    GREEN = "\033[32m"
    YELLOW = "\033[33m"
    BLUE = "\033[34m"
    MAGENTA = "\033[35m"
    CYAN = "\033[36m"
    WHITE = "\033[37m"
    GRAY = "\033[90m"

    # Bright colors
    BRIGHT_GREEN = "\033[92m"
    BRIGHT_YELLOW = "\033[93m"
    BRIGHT_BLUE = "\033[94m"
    BRIGHT_MAGENTA = "\033[95m"
    BRIGHT_CYAN = "\033[96m"


def get_terminal_width() -> int:
    """Get terminal width, default to 80."""
    return shutil.get_terminal_size((80, 24)).columns


def print_header():
    """Print the application header."""
    width = min(get_terminal_width(), 70)

    print(f"\n{Colors.BRIGHT_CYAN}{Colors.BOLD}")
    print("╔" + "═" * (width - 2) + "╗")
    print("║" + "🧠 Loom Chat Agent".center(width - 2) + "║")
    print("║" + "Interactive AI with Cognitive Loop".center(width - 2) + "║")
    print("╚" + "═" * (width - 2) + "╝")
    print(f"{Colors.RESET}")


def print_divider(char="─", color=Colors.GRAY):
    """Print a divider line."""
    width = min(get_terminal_width(), 70)
    print(f"{color}{char * width}{Colors.RESET}")


def print_box(title: str, content: str, color=Colors.CYAN):
    """Print content in a box."""
    width = min(get_terminal_width(), 70) - 4
    lines = []
    for line in content.split('\n'):
        while len(line) > width:
            lines.append(line[:width])
            line = line[width:]
        lines.append(line)

    print(f"{color}┌─ {title} " + "─" * max(0, width - len(title) - 1) + "┐" + Colors.RESET)
    for line in lines:
        print(f"{color}│{Colors.RESET} {line.ljust(width)} {color}│{Colors.RESET}")
    print(f"{color}└" + "─" * (width + 2) + "┘{Colors.RESET}")


def wrap_text(text: str, width: int, indent: str = "") -> str:
    """Wrap text to fit within width with optional indent."""
    words = text.split()
    lines = []
    current_line = indent

    for word in words:
        if len(current_line) + len(word) + 1 <= width:
            if current_line == indent:
                current_line += word
            else:
                current_line += " " + word
        else:
            if current_line != indent:
                lines.append(current_line)
            current_line = indent + word

    if current_line != indent:
        lines.append(current_line)

    return "\n".join(lines) if lines else indent


class ChatAgent:
    """Interactive chat agent with cognitive loop."""

    def __init__(self, config_path: str = None):
        self.config_path = Path(config_path) if config_path else None
        self.agent = None
        self.cognitive = None
        self.conversation_history = []
        self.verbose = True  # Show thinking process

    async def start(self, bridge_addr: str = None):
        """Initialize and start the agent."""
        # Load project config
        start_dir = self.config_path.parent if self.config_path else Path(".")
        project_config = load_project_config(start_dir)

        # Use provided address or from config
        addr = bridge_addr or project_config.bridge.address

        # Create base agent
        self.agent = Agent(
            agent_id="chat-assistant",
            topics=["chat.input", "chat.replies"],
            address=addr,
        )
        await self.agent.start()

        # Create LLM provider from config
        llm = LLMProvider.from_config(
            self.agent._ctx,
            project_config.agents.get("chat-assistant", {}).get("llm_provider", "deepseek"),
            project_config,
        )

        # Determine thinking strategy
        strategy_name = project_config.agents.get("chat-assistant", {}).get(
            "thinking_strategy", "react"
        )
        strategy = {
            "react": ThinkingStrategy.REACT,
            "single_shot": ThinkingStrategy.SINGLE_SHOT,
            "chain_of_thought": ThinkingStrategy.CHAIN_OF_THOUGHT,
        }.get(strategy_name, ThinkingStrategy.REACT)

        max_iterations = project_config.agents.get("chat-assistant", {}).get("max_iterations", 10)

        # Create cognitive agent
        self.cognitive = CognitiveAgent(
            ctx=self.agent._ctx,
            llm=llm,
            config=CognitiveConfig(
                system_prompt="""You are a helpful AI assistant with access to tools.

Available tools:
- weather:get: Get current weather. Args: {"location": "city name"}
- system:shell: Run shell commands. Args: {"command": "cmd"} (limited to: ls, echo, cat, grep)
- fs:read_file: Read file contents. Args: {"path": "file path"}

When you need information, use the appropriate tool.
Think step by step and explain your reasoning.
Be helpful, concise, and friendly.""",
                thinking_strategy=strategy,
                max_iterations=max_iterations,
                temperature=0.7,
            ),
            available_tools=["weather:get", "system:shell", "fs:read_file"],
        )

        return self

    async def chat(self, message: str, stream_callback=None) -> dict:
        """Process a chat message and return response with reasoning steps.

        Args:
            message: User's input message
            stream_callback: Optional callback for streaming updates

        Returns:
            dict with 'answer', 'steps', 'iterations', 'success'
        """
        # Add to conversation history
        self.conversation_history.append({"role": "user", "content": message})

        # Build context from history
        context = []
        if len(self.conversation_history) > 1:
            # Include recent history as context
            for msg in self.conversation_history[-6:-1]:  # Last 5 messages before current
                context.append(f"{msg['role'].capitalize()}: {msg['content']}")

        # Run cognitive loop
        result = await self.cognitive.run(message, context=context if context else None)

        # Add response to history
        self.conversation_history.append({"role": "assistant", "content": result.answer})

        return {
            "answer": result.answer,
            "steps": [
                {
                    "step": s.step,
                    "reasoning": s.reasoning,
                    "tool_call": s.tool_call.to_dict() if s.tool_call else None,
                    "observation": {
                        "success": s.observation.success,
                        "output": s.observation.output,
                        "error": s.observation.error,
                    }
                    if s.observation
                    else None,
                }
                for s in result.steps
            ],
            "iterations": result.iterations,
            "success": result.success,
            "latency_ms": result.total_latency_ms,
        }

    async def stop(self):
        """Stop the agent."""
        if self.agent:
            await self.agent.stop()

    def clear_history(self):
        """Clear conversation history."""
        self.conversation_history = []
        if self.cognitive:
            self.cognitive.memory.clear()


# ============================================================================
# CLI Interface
# ============================================================================

def print_thinking_step(step: dict, step_num: int):
    """Print a thinking step with nice formatting."""
    width = min(get_terminal_width(), 70)

    # Step header
    print(f"\n{Colors.BRIGHT_MAGENTA}  ┌─ Step {step_num} {'─' * (width - 15)}┐{Colors.RESET}")

    # Reasoning (Thought)
    if step.get("reasoning"):
        reasoning = step["reasoning"]
        print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET}")
        print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET} {Colors.YELLOW}💭 Thought:{Colors.RESET}")
        # Wrap reasoning text
        wrapped = wrap_text(reasoning, width - 8, "     ")
        for line in wrapped.split('\n'):
            print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET}{Colors.DIM}{line}{Colors.RESET}")

    # Tool Call (Action)
    if step.get("tool_call"):
        tc = step["tool_call"]
        print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET}")
        print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET} {Colors.CYAN}🔧 Action:{Colors.RESET} {Colors.BOLD}{tc.get('tool', 'unknown')}{Colors.RESET}")
        args_str = str(tc.get('args', {}))
        if len(args_str) > width - 15:
            args_str = args_str[:width - 18] + "..."
        print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET}    {Colors.DIM}Args: {args_str}{Colors.RESET}")

    # Observation (Result)
    if step.get("observation"):
        obs = step["observation"]
        print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET}")
        if obs.get("success"):
            output = obs.get("output", "")
            # Truncate long output
            if len(output) > 200:
                output = output[:200] + "..."
            print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET} {Colors.GREEN}✅ Observation:{Colors.RESET}")
            for line in output.split('\n')[:5]:  # Max 5 lines
                print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET}    {Colors.DIM}{line[:width-8]}{Colors.RESET}")
        else:
            error = obs.get("error", "Unknown error")
            print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET} {Colors.RED}❌ Error: {error[:width-15]}{Colors.RESET}")

    print(f"{Colors.BRIGHT_MAGENTA}  └{'─' * (width - 4)}┘{Colors.RESET}")


def print_result(result: dict):
    """Print the final result."""
    width = min(get_terminal_width(), 70)

    print(f"\n{Colors.BRIGHT_GREEN}{'═' * width}{Colors.RESET}")
    print(f"{Colors.BRIGHT_GREEN}{Colors.BOLD}🤖 Assistant:{Colors.RESET}")
    print()

    # Word-wrap the answer
    answer = result["answer"]
    wrapped = wrap_text(answer, width - 2, "")
    print(f"{wrapped}")

    print()
    print(f"{Colors.BRIGHT_GREEN}{'─' * width}{Colors.RESET}")

    # Stats
    stats = []
    if result.get("iterations", 0) > 0:
        stats.append(f"⚡ {result['iterations']} iterations")
    if result.get("latency_ms"):
        stats.append(f"⏱️  {result['latency_ms']}ms")
    if result.get("success") is not None:
        status = "✅" if result["success"] else "❌"
        stats.append(f"{status} {'Success' if result['success'] else 'Failed'}")

    if stats:
        print(f"{Colors.DIM}{' │ '.join(stats)}{Colors.RESET}")


def print_help():
    """Print help message."""
    width = min(get_terminal_width(), 70)

    print(f"\n{Colors.CYAN}{'─' * width}{Colors.RESET}")
    print(f"{Colors.CYAN}{Colors.BOLD}Available Commands:{Colors.RESET}")
    print(f"  {Colors.YELLOW}/help{Colors.RESET}      - Show this help message")
    print(f"  {Colors.YELLOW}/clear{Colors.RESET}     - Clear conversation history")
    print(f"  {Colors.YELLOW}/history{Colors.RESET}   - Show conversation history")
    print(f"  {Colors.YELLOW}/verbose{Colors.RESET}   - Toggle verbose mode (show thinking)")
    print(f"  {Colors.YELLOW}/quit{Colors.RESET}      - Exit the chat")
    print(f"\n{Colors.DIM}Available Tools:{Colors.RESET}")
    print(f"  {Colors.DIM}• weather:get   - Get weather for a location{Colors.RESET}")
    print(f"  {Colors.DIM}• system:shell  - Run shell commands (ls, echo, cat, grep){Colors.RESET}")
    print(f"  {Colors.DIM}• fs:read_file  - Read file contents{Colors.RESET}")
    print(f"{Colors.CYAN}{'─' * width}{Colors.RESET}\n")


async def run_cli():
    """Run interactive CLI chat."""
    print_header()

    print(f"{Colors.DIM}Type {Colors.YELLOW}/help{Colors.DIM} for available commands{Colors.RESET}")
    print_divider()

    chat = ChatAgent()

    try:
        print(f"\n{Colors.YELLOW}⏳ Connecting to Loom Bridge...{Colors.RESET}")
        await chat.start()
        print(f"{Colors.GREEN}✅ Connected! Agent ready.{Colors.RESET}\n")
    except Exception as e:
        print(f"{Colors.RED}❌ Failed to connect: {e}{Colors.RESET}")
        print(f"{Colors.DIM}Make sure loom-bridge-server is running (loom run){Colors.RESET}")
        return

    try:
        while True:
            try:
                # Prompt
                user_input = input(f"{Colors.BRIGHT_BLUE}{Colors.BOLD}You ▶{Colors.RESET} ").strip()
            except EOFError:
                break
            except KeyboardInterrupt:
                print(f"\n{Colors.YELLOW}Use /quit to exit{Colors.RESET}")
                continue

            if not user_input:
                continue

            # Handle commands
            if user_input.startswith("/"):
                cmd = user_input.lower().split()[0]

                if cmd in ["/quit", "/exit", "/q"]:
                    print(f"\n{Colors.CYAN}Goodbye! 👋{Colors.RESET}\n")
                    break

                if cmd == "/clear":
                    chat.clear_history()
                    print(f"{Colors.GREEN}✅ Conversation cleared.{Colors.RESET}\n")
                    continue

                if cmd == "/help":
                    print_help()
                    continue

                if cmd == "/verbose":
                    chat.verbose = not chat.verbose
                    state = "ON" if chat.verbose else "OFF"
                    print(f"{Colors.GREEN}Verbose mode: {state}{Colors.RESET}\n")
                    continue

                if cmd == "/history":
                    if not chat.conversation_history:
                        print(f"{Colors.DIM}No conversation history yet.{Colors.RESET}\n")
                    else:
                        print(f"\n{Colors.CYAN}📜 Conversation History:{Colors.RESET}")
                        for i, msg in enumerate(chat.conversation_history):
                            role_color = Colors.BRIGHT_BLUE if msg["role"] == "user" else Colors.BRIGHT_GREEN
                            role = "You" if msg["role"] == "user" else "AI"
                            content = msg["content"][:60] + "..." if len(msg["content"]) > 60 else msg["content"]
                            print(f"  {Colors.DIM}[{i+1}]{Colors.RESET} {role_color}{role}:{Colors.RESET} {content}")
                        print()
                    continue

                print(f"{Colors.RED}Unknown command: {cmd}. Type /help for available commands.{Colors.RESET}\n")
                continue

            # Process message
            print(f"\n{Colors.MAGENTA}🧠 Processing...{Colors.RESET}")

            try:
                result = await chat.chat(user_input)

                # Show reasoning steps if verbose mode
                if chat.verbose and result.get("steps"):
                    print(f"\n{Colors.BRIGHT_MAGENTA}{Colors.BOLD}💭 Thinking Process:{Colors.RESET}")
                    for i, step in enumerate(result["steps"], 1):
                        print_thinking_step(step, i)

                # Show final result
                print_result(result)
                print()

            except Exception as e:
                print(f"{Colors.RED}❌ Error: {e}{Colors.RESET}")
                import traceback
                traceback.print_exc()
                print()

    except KeyboardInterrupt:
        print(f"\n{Colors.CYAN}Goodbye! 👋{Colors.RESET}\n")
    finally:
        await chat.stop()


if __name__ == "__main__":
    try:
        asyncio.run(run_cli())
    except KeyboardInterrupt:
        print(f"\n{Colors.CYAN}Goodbye! 👋{Colors.RESET}\n")
