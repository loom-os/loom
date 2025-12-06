# Loom Python Docs Index

## Core Guides

- **SDK Guide**: `SDK_GUIDE.md` - Build your first agent
- **LLM Provider Guide**: `LLM_GUIDE.md` - Configure LLM providers
- **Cognitive Loop Guide**: `COGNITIVE_GUIDE.md` - Implement reasoning patterns
- **CLI Guide**: `CLI_GUIDE.md` - Interactive chat interface ✨ NEW

## Context Engineering

- **Architecture**: `context/DESIGN.md` - Overall design and principles
- **Reduction**: `context/REDUCTION.md` - Step reduction strategies
- **Offloading**: `context/OFFLOADING.md` - Data offloading patterns
- **Isolation**: `context/ISOLATION.md` - Multi-agent context management
- **Integration**: `CONTEXT_INTEGRATION.md` - End-to-end integration guide ✨ NEW

## Project Docs

- Quickstart & Runtime: `../../docs/QUICKSTART.md`, `../../docs/BUILD_LOCAL.md`
- Architecture: `../../ARCHITECTURE.md`, `../../ROADMAP.md`

## Recent Updates (Dec 6, 2025)

### Context Engineering Integration v0.2.1

**Fixed**:

- ✅ CLI display now shows offload references correctly (`Step.observation` not `Step.outcome`)
- ✅ Step compaction integrated into prompt construction
- ✅ FINAL ANSWER parsing fixed (no false matches)
- ✅ Memory tracking includes offload references

**Added**:

- 📚 CLI Guide with context engineering details
- 📚 Context Integration Guide (complete pipeline walkthrough)
- 🧪 9 new integration tests (`TestCLIDisplay`, `TestEndToEndContextEngineering`)
- 🧪 6 new unit tests (`TestStepAttributes`, `TestThoughtStepIntegration`)
- 📊 Context metrics in CLI output (offload count)

**Test Coverage**: 64 passing tests (100% for context engineering)

Phase 2 focus: Enhance chat-assistant with deep research, workspace tools (fs:write/list/delete), agent spawning, web search integration, and report generation.
