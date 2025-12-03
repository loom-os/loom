# Tool Use Example for Voice Agent

This directory contains example code demonstrating how to integrate `web.search` and `weather.get` capabilities into the voice agent using the Tool Orchestrator.

## Overview

The voice agent can now use external tools to answer queries:

- **web.search**: Search the web using DuckDuckGo
- **weather.get**: Get current weather information using Open-Meteo API

## Architecture

```
User Voice Query
    ↓
STT (Speech-to-Text)
    ↓
Tool Orchestrator
    ├→ Discovers available tools from ActionBroker
    ├→ Sends query + tool schemas to LLM
    ├→ LLM decides to use tools or answer directly
    ├→ Parses tool calls from LLM response
    ├→ Invokes tools via ActionBroker
    └→ Returns final answer to user
    ↓
TTS (Text-to-Speech)
```

## Example Usage

### 1. Basic Integration (main.rs)

To add tool use capabilities to your voice agent, register the providers and use the ToolOrchestrator:

```rust
use loom_core::{WebSearchProvider, WeatherProvider};
use loom_core::llm::{ToolOrchestrator, OrchestratorOptions, ToolChoice};
use loom_core::context::{PromptBundle, TokenBudget};

// Register capability providers
broker.register_provider(Arc::new(WebSearchProvider::new()));
broker.register_provider(Arc::new(WeatherProvider::new()));

// Create LLM client and orchestrator
let llm_client = Arc::new(LlmClient::new(llm_config));
let mut orchestrator = ToolOrchestrator::new(llm_client, Arc::clone(&broker));

// Process query with tools
let bundle = PromptBundle {
    system: "You are a helpful voice assistant with access to web search and weather information.".into(),
    instructions: user_query,
    tools_json_schema: None,
    context_docs: vec![],
    history: vec![],
};

let options = OrchestratorOptions {
    tool_choice: ToolChoice::Auto,
    per_tool_timeout_ms: 30_000,
    refine_on_tool_result: true,
    max_tools_exposed: 64,
};

let answer = orchestrator.run(&bundle, Some(budget), options, Some(correlation_id)).await?;
println!("Answer: {}", answer.text);
```

### 2. Example Queries

The voice agent can now handle queries like:

- **Weather**: "What's the weather in London?"

  - Tool used: `weather.get`
  - Parameters: `{"location": "London", "units": "celsius"}`

- **Web Search**: "Search for information about Rust programming"

  - Tool used: `web.search`
  - Parameters: `{"query": "Rust programming", "top_k": 5}`

- **Combined**: "What's the weather in San Francisco and search for best restaurants there"
  - Tools used: `weather.get` + `web.search`
  - Multiple tool calls executed sequentially

## Configuration

### Environment Variables

No API keys required! Both providers use free public APIs:

- **DuckDuckGo** for web search (no API key needed)
- **Open-Meteo** for weather (no API key needed)

### Custom Configuration

You can customize the providers:

```rust
use loom_core::providers::{WebSearchConfig, WeatherConfig};

// Custom web search config
let web_config = WebSearchConfig {
    api_endpoint: "https://api.duckduckgo.com/".to_string(),
    timeout_ms: 15_000,
    user_agent: "my-agent/1.0".to_string(),
};
broker.register_provider(Arc::new(WebSearchProvider::with_config(web_config)));

// Custom weather config
let weather_config = WeatherConfig {
    api_endpoint: "https://api.open-meteo.com/v1/forecast".to_string(),
    geocoding_endpoint: "https://geocoding-api.open-meteo.com/v1/search".to_string(),
    timeout_ms: 15_000,
    user_agent: "my-agent/1.0".to_string(),
};
broker.register_provider(Arc::new(WeatherProvider::with_config(weather_config)));
```

## Observability

The Tool Orchestrator includes built-in observability:

```rust
// After running queries, check statistics
println!("Total tool invocations: {}", orchestrator.stats.total_invocations);
println!("Total tool calls: {}", orchestrator.stats.total_tool_calls);
println!("Tool errors: {}", orchestrator.stats.total_tool_errors);
println!("Avg latency: {:.2}ms", orchestrator.stats.avg_tool_latency_ms);
```

Tracing logs are emitted at the `tool_orch` target:

```rust
RUST_LOG=tool_orch=debug,action_broker=debug cargo run
```

## Testing

Run the integration tests to verify tool use:

```bash
cd core
cargo test --test integration_test e2e_tool_use
```

Run specific tests for real providers:

```bash
cargo test --test integration_test test_real_web_search_provider
cargo test --test integration_test test_real_weather_provider
```

## Implementation Details

### Tool Discovery

The orchestrator automatically discovers available tools from the ActionBroker:

1. Calls `broker.list_capabilities()`
2. Extracts tool schemas from capability metadata
3. Formats them as OpenAI-compatible function schemas
4. Includes them in the LLM request

### Tool Invocation

When the LLM returns tool calls:

1. Orchestrator parses the tool calls from the response
2. Converts them to `ActionCall` messages
3. Invokes each tool via the ActionBroker
4. Collects results and errors
5. Optionally refines the answer with tool results

### Error Handling

The system handles various error scenarios:

- **Missing parameters**: Returns `ActionError` with code `INVALID_QUERY` or `INVALID_LOCATION`
- **Network failures**: Returns `ActionError` with code `SEARCH_FAILED` or `WEATHER_FETCH_FAILED`
- **Timeouts**: ActionBroker enforces timeouts and returns `ActionTimeout` status
- **Malformed responses**: Gracefully falls back to empty results

## Full Example

See `tool_use_example.rs` for a complete standalone example that doesn't require the full voice pipeline.

## Next Steps

To extend the system with more tools:

1. Create a new provider implementing `CapabilityProvider`
2. Include JSON schema in the descriptor metadata
3. Register with the ActionBroker
4. The orchestrator will automatically discover and use it

Example tools to add:

- Calendar operations
- Email/messaging
- File operations
- Database queries
- Custom APIs
