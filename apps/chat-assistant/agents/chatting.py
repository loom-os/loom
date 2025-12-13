#!/usr/bin/env python3
"""Chat Agent Backend - Cognitive AI service.

This is a simple chat backend using the reusable BackendAgent infrastructure.
All the complex event handling, streaming, and cognitive setup is handled by
the infra layer in loom.runtime.BackendAgent.

Run with:
    loom run           # Start runtime + this backend agent
    loom chat          # In another terminal, start client UI
"""

import asyncio
import os
import sys
from pathlib import Path


def _load_dotenv():
    """Load .env file if present."""
    for env_path in [Path(__file__).parent.parent / ".env", Path(".env")]:
        if env_path.exists():
            for line in env_path.read_text().splitlines():
                line = line.strip()
                if line and not line.startswith("#") and "=" in line:
                    key, _, value = line.partition("=")
                    key = key.strip()
                    value = value.strip().strip("'\"")
                    if key and key not in os.environ:
                        os.environ[key] = value
            break


_load_dotenv()

# Add loom-py to path for local development
sys.path.insert(0, str(Path(__file__).parent.parent.parent.parent / "loom-py" / "src"))


async def main():
    """Start the chat backend agent."""
    from loom.runtime import BackendAgent

    project_dir = Path(__file__).parent.parent

    print("=" * 60)
    print("🧠 Chat Backend Agent")
    print("=" * 60)
    print(f"📁 Project: {project_dir}")

    try:
        backend = await BackendAgent.from_config(
            agent_id="chat-assistant",
            project_dir=project_dir,
            verbose=True,
        )
        await backend.run()

    except ValueError as e:
        print(f"❌ Configuration error: {e}")
        return 1
    except ConnectionError as e:
        print(f"❌ Connection error: {e}")
        print("   Make sure runtime is running (loom up or loom run)")
        return 1
    except Exception as e:
        print(f"❌ Unexpected error: {e}")
        import traceback
        traceback.print_exc()
        return 1

    return 0


if __name__ == "__main__":
    try:
        exit_code = asyncio.run(main())
        sys.exit(exit_code or 0)
    except KeyboardInterrupt:
        print("\n[Backend] Interrupted by user.")
        sys.exit(0)
