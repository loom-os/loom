"""Loom Chat CLI - Interactive chat with running agents.

This module provides terminal UI for chatting with cognitive agents.
"""

from __future__ import annotations

import shutil
from pathlib import Path
from typing import TYPE_CHECKING, Optional

if TYPE_CHECKING:
    from ..cognitive import CognitiveAgent


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
    print("║" + "🧠 Loom Chat".center(width - 2) + "║")
    print("║" + "Interactive AI with Cognitive Loop".center(width - 2) + "║")
    print("╚" + "═" * (width - 2) + "╝")
    print(f"{Colors.RESET}")


def print_divider(char="─", color=Colors.GRAY):
    """Print a divider line."""
    width = min(get_terminal_width(), 70)
    print(f"{color}{char * width}{Colors.RESET}")


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
        wrapped = wrap_text(reasoning, width - 8, "     ")
        for line in wrapped.split("\n"):
            print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET}{Colors.DIM}{line}{Colors.RESET}")

    # Tool Call (Action)
    if step.get("tool_call"):
        tc = step["tool_call"]
        print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET}")
        print(
            f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET} {Colors.CYAN}🔧 Action:{Colors.RESET} "
            f"{Colors.BOLD}{tc.get('tool', 'unknown')}{Colors.RESET}"
        )
        args_str = str(tc.get("args", {}))
        if len(args_str) > width - 15:
            args_str = args_str[: width - 18] + "..."
        print(
            f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET}    {Colors.DIM}Args: {args_str}{Colors.RESET}"
        )

    # Observation (Result)
    if step.get("observation"):
        obs = step["observation"]
        print(f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET}")
        if obs.get("success"):
            output = obs.get("output", "")
            if len(output) > 200:
                output = output[:200] + "..."
            print(
                f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET} {Colors.GREEN}✅ Observation:{Colors.RESET}"
            )
            for line in output.split("\n")[:5]:
                print(
                    f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET}    "
                    f"{Colors.DIM}{line[:width-8]}{Colors.RESET}"
                )
        else:
            error = obs.get("error", "Unknown error")
            print(
                f"{Colors.BRIGHT_MAGENTA}  │{Colors.RESET} "
                f"{Colors.RED}❌ Error: {error[:width-15]}{Colors.RESET}"
            )

    print(f"{Colors.BRIGHT_MAGENTA}  └{'─' * (width - 4)}┘{Colors.RESET}")


def print_result(result: dict):
    """Print the final result."""
    width = min(get_terminal_width(), 70)

    print(f"\n{Colors.BRIGHT_GREEN}{'═' * width}{Colors.RESET}")
    print(f"{Colors.BRIGHT_GREEN}{Colors.BOLD}🤖 Assistant:{Colors.RESET}")
    print()

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
    print(f"  {Colors.YELLOW}/stream{Colors.RESET}    - Toggle streaming mode")
    print(
        f"  {Colors.YELLOW}/research{Colors.RESET}  - Deep research mode (e.g., /research AI frameworks)"
    )
    print(f"  {Colors.YELLOW}/quit{Colors.RESET}      - Exit the chat")
    print(f"\n{Colors.DIM}Available Tools:{Colors.RESET}")
    print(f"  {Colors.DIM}• weather:get    - Get weather for a location{Colors.RESET}")
    print(
        f"  {Colors.DIM}• system:shell   - Run shell commands (ls, echo, cat, grep){Colors.RESET}"
    )
    print(f"  {Colors.DIM}• fs:read_file   - Read file contents{Colors.RESET}")
    print(f"  {Colors.DIM}• fs:write_file  - Write content to a file{Colors.RESET}")
    print(f"  {Colors.DIM}• fs:list_dir    - List directory contents{Colors.RESET}")
    print(f"  {Colors.DIM}• fs:delete      - Delete a file or empty directory{Colors.RESET}")
    print(f"{Colors.CYAN}{'─' * width}{Colors.RESET}\n")


def print_streaming_header():
    """Print streaming header."""
    print(f"\n{Colors.BRIGHT_MAGENTA}{Colors.BOLD}💭 Thinking...{Colors.RESET}")
    print(f"{Colors.DIM}─" * 50 + f"{Colors.RESET}")


def print_stream_step_complete(step):
    """Print a brief note when a thinking step completes."""
    width = min(get_terminal_width(), 70)
    print()  # Newline after streamed content

    if step.tool_call:
        tool_name = step.tool_call.name
        print(f"\n{Colors.CYAN}🔧 Calling tool: {Colors.BOLD}{tool_name}{Colors.RESET}")

        # Show observation
        if step.observation:
            if step.observation.success:
                output = step.observation.output
                if len(output) > 150:
                    output = output[:150] + "..."
                print(f"{Colors.GREEN}   ✅ Result:{Colors.RESET}")
                for line in output.split("\n")[:3]:
                    print(f"{Colors.DIM}      {line[:width-10]}{Colors.RESET}")
            else:
                print(f"{Colors.RED}   ❌ Error: {step.observation.error}{Colors.RESET}")
        print()


# ============================================================================
# Chat Session
# ============================================================================


class ChatSession:
    """Interactive chat session with a cognitive agent."""

    def __init__(
        self,
        agent_id: str = "chat-assistant",
        bridge_addr: Optional[str] = None,
        verbose: bool = True,
        streaming: bool = True,
    ):
        self.agent_id = agent_id
        self.bridge_addr = bridge_addr
        self.verbose = verbose
        self.streaming = streaming
        self.agent = None
        self.cognitive: Optional[CognitiveAgent] = None
        self.conversation_history: list[dict] = []

    async def start(self):
        """Initialize and start the chat session."""
        from .. import Agent, CognitiveAgent, CognitiveConfig, ThinkingStrategy
        from ..llm import LLMProvider
        from ..runtime.config import load_project_config

        # Load project config
        project_config = load_project_config(Path.cwd())

        # Use provided address or from config
        addr = self.bridge_addr or project_config.bridge.address

        # Create base agent
        self.agent = Agent(
            agent_id=self.agent_id,
            topics=["chat.input", "chat.replies"],
            address=addr,
        )
        await self.agent.start()

        # Create LLM provider from config
        llm = LLMProvider.from_config(
            self.agent._ctx,
            project_config.agents.get(self.agent_id, {}).get("llm_provider", "deepseek"),
            project_config,
        )

        # Determine thinking strategy
        strategy_name = project_config.agents.get(self.agent_id, {}).get(
            "thinking_strategy", "react"
        )
        strategy = {
            "react": ThinkingStrategy.REACT,
            "single_shot": ThinkingStrategy.SINGLE_SHOT,
            "chain_of_thought": ThinkingStrategy.CHAIN_OF_THOUGHT,
        }.get(strategy_name, ThinkingStrategy.REACT)

        max_iterations = project_config.agents.get(self.agent_id, {}).get("max_iterations", 10)

        # Create cognitive agent with permission callback for human-in-the-loop
        self.cognitive = CognitiveAgent(
            ctx=self.agent._ctx,
            llm=llm,
            config=CognitiveConfig(
                system_prompt="""You are a helpful AI assistant with access to tools.

Available tools:
- weather:get: Get current weather. Args: {"location": "city name"}
- system:shell: Run shell commands. Args: {"command": "cmd"} (some commands may require user approval)
- fs:read_file: Read file contents. Args: {"path": "relative/path"}
- fs:write_file: Write content to file. Args: {"path": "relative/path", "content": "text"}
- fs:list_dir: List directory. Args: {"path": "relative/path"} (optional, defaults to workspace root)
- fs:delete: Delete file or empty directory. Args: {"path": "relative/path"}

When you need information, use the appropriate tool.
Think step by step and explain your reasoning.
Be helpful, concise, and friendly.""",
                thinking_strategy=strategy,
                max_iterations=max_iterations,
                temperature=0.7,
            ),
            available_tools=[
                "weather:get",
                "system:shell",
                "fs:read_file",
                "fs:write_file",
                "fs:list_dir",
                "fs:delete",
            ],
            permission_callback=self._request_permission,
        )

        return self

    def _request_permission(self, tool_name: str, args: dict, error_msg: str) -> bool:
        """Request user permission for a denied tool action.

        This is called when a tool (e.g., shell command) is denied by the sandbox.
        Returns True if user approves, False otherwise.
        """
        print()
        print(f"{Colors.YELLOW}{'─' * 50}{Colors.RESET}")
        print(f"{Colors.YELLOW}⚠️  Permission Required{Colors.RESET}")
        print(f"{Colors.DIM}Tool: {tool_name}{Colors.RESET}")
        print(f"{Colors.DIM}Args: {args}{Colors.RESET}")
        print(f"{Colors.DIM}Reason: {error_msg}{Colors.RESET}")
        print(f"{Colors.YELLOW}{'─' * 50}{Colors.RESET}")

        try:
            response = (
                input(f"{Colors.BRIGHT_YELLOW}Allow this action? [y/N]: {Colors.RESET}")
                .strip()
                .lower()
            )
            approved = response in ("y", "yes")

            if approved:
                print(f"{Colors.GREEN}✅ Approved by user{Colors.RESET}")
            else:
                print(f"{Colors.RED}❌ Denied by user{Colors.RESET}")

            return approved
        except (EOFError, KeyboardInterrupt):
            print(f"\n{Colors.RED}❌ Denied (interrupted){Colors.RESET}")
            return False

    async def chat(self, message: str) -> dict:
        """Process a chat message and return response with reasoning steps."""
        if not self.cognitive:
            raise RuntimeError("Chat session not started")

        # Add to conversation history
        self.conversation_history.append({"role": "user", "content": message})

        # Build context from history
        context = []
        if len(self.conversation_history) > 1:
            for msg in self.conversation_history[-6:-1]:
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
                    "observation": (
                        {
                            "success": s.observation.success,
                            "output": s.observation.output,
                            "error": s.observation.error,
                        }
                        if s.observation
                        else None
                    ),
                }
                for s in result.steps
            ],
            "iterations": result.iterations,
            "success": result.success,
            "latency_ms": result.total_latency_ms,
        }

    async def chat_stream(self, message: str, on_chunk=None, on_step=None):
        """Process a chat message with streaming output.

        Args:
            message: User input message
            on_chunk: Callback for LLM text chunks (str)
            on_step: Callback for complete thinking steps (ThoughtStep)

        Returns:
            Final result dict
        """
        from ..cognitive.types import CognitiveResult, ThoughtStep

        if not self.cognitive:
            raise RuntimeError("Chat session not started")

        # Add to conversation history
        self.conversation_history.append({"role": "user", "content": message})

        # Build context from history
        context = []
        if len(self.conversation_history) > 1:
            for msg in self.conversation_history[-6:-1]:
                context.append(f"{msg['role'].capitalize()}: {msg['content']}")

        # Stream cognitive loop
        final_result = None
        steps = []

        async for item in self.cognitive.run_stream(message, context=context if context else None):
            if isinstance(item, str):
                # LLM text chunk
                if on_chunk:
                    on_chunk(item)
            elif isinstance(item, ThoughtStep):
                # Complete thinking step
                steps.append(item)
                if on_step:
                    on_step(item)
            elif isinstance(item, CognitiveResult):
                # Final result
                final_result = item

        if final_result:
            # Add response to history
            self.conversation_history.append({"role": "assistant", "content": final_result.answer})

            return {
                "answer": final_result.answer,
                "steps": [
                    {
                        "step": s.step,
                        "reasoning": s.reasoning,
                        "tool_call": s.tool_call.to_dict() if s.tool_call else None,
                        "observation": (
                            {
                                "success": s.observation.success,
                                "output": s.observation.output,
                                "error": s.observation.error,
                            }
                            if s.observation
                            else None
                        ),
                    }
                    for s in final_result.steps
                ],
                "iterations": final_result.iterations,
                "success": final_result.success,
                "latency_ms": final_result.total_latency_ms,
            }

        return {"answer": "", "steps": [], "iterations": 0, "success": False}

    async def stop(self):
        """Stop the chat session."""
        if self.agent:
            await self.agent.stop()

    def clear_history(self):
        """Clear conversation history."""
        self.conversation_history = []
        if self.cognitive:
            self.cognitive.memory.clear()

    async def research(self, topic: str, on_progress=None) -> dict:
        """Deep research mode: multi-step investigation on a topic.

        This method:
        1. Plans research approach
        2. Gathers information using tools
        3. Synthesizes findings
        4. Saves report to workspace/reports/

        Args:
            topic: Research topic/question
            on_progress: Callback for progress updates (str)

        Returns:
            Dict with report path and summary
        """
        from datetime import datetime

        if not self.cognitive:
            raise RuntimeError("Chat session not started")

        def log(msg: str):
            if on_progress:
                on_progress(msg)

        log(f"📚 Starting deep research on: {topic}")

        # Phase 1: Plan research
        log("📋 Phase 1: Planning research approach...")
        plan_prompt = f"""Plan a research approach for: {topic}

Create a brief research plan with 3-5 specific questions to investigate.
Format as a numbered list."""

        plan_result = await self.cognitive.run(plan_prompt)
        research_plan = plan_result.answer
        log("   ✅ Research plan created")

        # Phase 2: Investigate each question
        log("🔍 Phase 2: Investigating questions...")
        findings = []

        # Extract questions from plan and investigate
        investigate_prompt = f"""Based on this research plan:
{research_plan}

Now investigate the topic: {topic}

Use available tools (web search, file reading) to gather information.
Provide detailed findings with sources where possible."""

        investigate_result = await self.cognitive.run(investigate_prompt)
        findings.append(investigate_result.answer)
        log(f"   ✅ Investigation complete ({investigate_result.iterations} iterations)")

        # Phase 3: Synthesize findings
        log("📝 Phase 3: Synthesizing report...")
        synthesis_prompt = f"""Synthesize these research findings into a comprehensive report:

Topic: {topic}

Plan:
{research_plan}

Findings:
{chr(10).join(findings)}

Create a well-structured markdown report with:
1. Executive Summary
2. Key Findings
3. Detailed Analysis
4. Conclusions
5. References (if any)"""

        synthesis_result = await self.cognitive.run(synthesis_prompt)
        report_content = synthesis_result.answer

        # Phase 4: Save report
        log("💾 Phase 4: Saving report...")
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        safe_topic = "".join(c if c.isalnum() or c in "-_ " else "_" for c in topic)[:50]
        report_filename = f"workspace/reports/{timestamp}_{safe_topic}.md"

        # Create full report with metadata
        full_report = f"""# Research Report: {topic}

**Generated**: {datetime.now().strftime("%Y-%m-%d %H:%M:%S")}
**Agent**: {self.agent_id}

---

{report_content}

---

## Research Metadata

- **Topic**: {topic}
- **Total Iterations**: {plan_result.iterations + investigate_result.iterations + synthesis_result.iterations}
- **Research Plan**: {len(research_plan)} chars
- **Findings**: {len(chr(10).join(findings))} chars
"""

        # Save using fs:write_file tool
        try:
            await self.cognitive.ctx.tool(
                "fs:write_file",
                payload={"path": report_filename, "content": full_report},
            )
            log(f"   ✅ Report saved to: {report_filename}")
        except Exception as e:
            log(f"   ⚠️ Could not save report: {e}")
            report_filename = None

        return {
            "topic": topic,
            "report_path": report_filename,
            "summary": (
                report_content[:500] + "..." if len(report_content) > 500 else report_content
            ),
            "full_report": full_report,
            "iterations": plan_result.iterations
            + investigate_result.iterations
            + synthesis_result.iterations,
        }


# ============================================================================
# CLI Runner
# ============================================================================


async def run_chat_cli(bridge_addr: Optional[str] = None, agent_id: str = "chat-assistant"):
    """Run interactive CLI chat."""
    print_header()

    print(f"{Colors.DIM}Type {Colors.YELLOW}/help{Colors.DIM} for available commands{Colors.RESET}")
    print_divider()

    session = ChatSession(agent_id=agent_id, bridge_addr=bridge_addr)

    try:
        print(f"\n{Colors.YELLOW}⏳ Connecting to Loom Bridge...{Colors.RESET}")
        await session.start()
        print(f"{Colors.GREEN}✅ Connected! Agent ready.{Colors.RESET}\n")
    except Exception as e:
        print(f"{Colors.RED}❌ Failed to connect: {e}{Colors.RESET}")
        print(f"{Colors.DIM}Make sure Loom runtime is running (loom run or loom up){Colors.RESET}")
        return 1

    try:
        while True:
            try:
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
                    session.clear_history()
                    print(f"{Colors.GREEN}✅ Conversation cleared.{Colors.RESET}\n")
                    continue

                if cmd == "/help":
                    print_help()
                    continue

                if cmd == "/verbose":
                    session.verbose = not session.verbose
                    state = "ON" if session.verbose else "OFF"
                    print(f"{Colors.GREEN}Verbose mode: {state}{Colors.RESET}\n")
                    continue

                if cmd == "/history":
                    if not session.conversation_history:
                        print(f"{Colors.DIM}No conversation history yet.{Colors.RESET}\n")
                    else:
                        print(f"\n{Colors.CYAN}📜 Conversation History:{Colors.RESET}")
                        for i, msg in enumerate(session.conversation_history):
                            role_color = (
                                Colors.BRIGHT_BLUE if msg["role"] == "user" else Colors.BRIGHT_GREEN
                            )
                            role = "You" if msg["role"] == "user" else "AI"
                            content = (
                                msg["content"][:60] + "..."
                                if len(msg["content"]) > 60
                                else msg["content"]
                            )
                            print(
                                f"  {Colors.DIM}[{i+1}]{Colors.RESET} "
                                f"{role_color}{role}:{Colors.RESET} {content}"
                            )
                        print()
                    continue

                if cmd == "/stream":
                    session.streaming = not session.streaming
                    state = "ON" if session.streaming else "OFF"
                    print(f"{Colors.GREEN}Streaming mode: {state}{Colors.RESET}\n")
                    continue

                if cmd == "/research":
                    # Extract topic from command
                    parts = user_input.split(maxsplit=1)
                    if len(parts) < 2:
                        print(f"{Colors.RED}Usage: /research <topic>{Colors.RESET}")
                        print(f"{Colors.DIM}Example: /research AI agent frameworks{Colors.RESET}\n")
                        continue

                    topic = parts[1].strip()
                    print(
                        f"\n{Colors.BRIGHT_MAGENTA}{Colors.BOLD}🔬 Deep Research Mode{Colors.RESET}"
                    )
                    print(f"{Colors.DIM}Topic: {topic}{Colors.RESET}")
                    print_divider()

                    def progress_callback(msg: str):
                        print(f"{Colors.CYAN}{msg}{Colors.RESET}")

                    try:
                        result = await session.research(topic, on_progress=progress_callback)
                        print()
                        print(f"{Colors.BRIGHT_GREEN}{'═' * 50}{Colors.RESET}")
                        print(
                            f"{Colors.BRIGHT_GREEN}{Colors.BOLD}📊 Research Complete{Colors.RESET}"
                        )
                        print()
                        if result.get("report_path"):
                            print(
                                f"{Colors.GREEN}📄 Report saved: {result['report_path']}{Colors.RESET}"
                            )
                        print(f"{Colors.DIM}Total iterations: {result['iterations']}{Colors.RESET}")
                        print()
                        print(f"{Colors.BOLD}Summary:{Colors.RESET}")
                        print(result["summary"])
                        print(f"{Colors.BRIGHT_GREEN}{'═' * 50}{Colors.RESET}\n")
                    except Exception as e:
                        print(f"{Colors.RED}❌ Research failed: {e}{Colors.RESET}\n")
                        import traceback

                        traceback.print_exc()
                    continue

                print(
                    f"{Colors.RED}Unknown command: {cmd}. "
                    f"Type /help for available commands.{Colors.RESET}\n"
                )
                continue

            # Process message
            try:
                if session.streaming:
                    # Streaming mode - show LLM output in real-time
                    print_streaming_header()

                    # Callback to print chunks directly
                    def on_chunk(chunk: str):
                        print(f"{Colors.DIM}{chunk}{Colors.RESET}", end="", flush=True)

                    # Callback when a step completes
                    def on_step(step):
                        print_stream_step_complete(step)

                    result = await session.chat_stream(
                        user_input,
                        on_chunk=on_chunk,
                        on_step=on_step,
                    )

                    # Show final result (without repeating thinking if shown during streaming)
                    print()  # Newline after streaming content
                    print_result(result)
                    print()
                else:
                    # Non-streaming mode - wait for complete response
                    print(f"\n{Colors.MAGENTA}🧠 Processing...{Colors.RESET}")
                    result = await session.chat(user_input)

                    # Show reasoning steps if verbose mode
                    if session.verbose and result.get("steps"):
                        print(
                            f"\n{Colors.BRIGHT_MAGENTA}{Colors.BOLD}💭 Thinking Process:{Colors.RESET}"
                        )
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
        await session.stop()

    return 0
