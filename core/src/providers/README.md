# Capability Providers

This module contains built-in capability providers for common tools and services.

## Available Providers

### 1. WebSearchProvider (`web.search`)

Provides web search functionality using DuckDuckGo's Instant Answer API.

**Features:**

- No API key required
- Free public API
- Returns search results with titles, URLs, and snippets
- Configurable result limit (1-10)

**Usage:**

```rust
use loom_core::WebSearchProvider;
use std::sync::Arc;

let provider = Arc::new(WebSearchProvider::new());
broker.register_provider(provider);
```

**Tool Schema:**

```json
{
  "type": "object",
  "properties": {
    "query": {
      "type": "string",
      "description": "Search query string"
    },
    "top_k": {
      "type": "integer",
      "description": "Maximum number of results to return",
      "minimum": 1,
      "maximum": 10,
      "default": 5
    }
  },
  "required": ["query"]
}
```

**Example Call:**

```json
{
  "query": "Rust programming language",
  "top_k": 3
}
```

**Example Response:**

```json
{
  "query": "Rust programming language",
  "results": [
    {
      "title": "Rust - A language empowering everyone",
      "url": "https://www.rust-lang.org/",
      "snippet": "A language empowering everyone to build reliable and efficient software."
    },
    {
      "title": "The Rust Programming Language",
      "url": "https://doc.rust-lang.org/book/",
      "snippet": null
    }
  ],
  "count": 2
}
```

### 2. WeatherProvider (`weather.get`)

Provides current weather information using Open-Meteo API.

**Features:**

- No API key required
- Free public API
- Global coverage with automatic geocoding
- Supports both Celsius and Fahrenheit
- Returns temperature, conditions, humidity, wind speed

**Usage:**

```rust
use loom_core::WeatherProvider;
use std::sync::Arc;

let provider = Arc::new(WeatherProvider::new());
broker.register_provider(provider);
```

**Tool Schema:**

```json
{
  "type": "object",
  "properties": {
    "location": {
      "type": "string",
      "description": "City or location name (e.g., 'Beijing', 'New York', 'London')"
    },
    "units": {
      "type": "string",
      "description": "Temperature units",
      "enum": ["celsius", "fahrenheit"],
      "default": "celsius"
    }
  },
  "required": ["location"]
}
```

**Example Call:**

```json
{
  "location": "Tokyo",
  "units": "celsius"
}
```

**Example Response:**

```json
{
  "location": "Tokyo, Japan",
  "temperature": 18.5,
  "conditions": "partly cloudy",
  "humidity": 65,
  "wind_speed": 12.5,
  "units": "celsius"
}
```

## Configuration

Both providers can be customized with configuration structs:

### WebSearchProvider Configuration

```rust
use loom_core::providers::{WebSearchProvider, WebSearchConfig};

let config = WebSearchConfig {
    api_endpoint: "https://api.duckduckgo.com/".to_string(),
    timeout_ms: 15_000,
    user_agent: "my-agent/1.0".to_string(),
};

let provider = WebSearchProvider::with_config(config);
```

### WeatherProvider Configuration

```rust
use loom_core::providers::{WeatherProvider, WeatherConfig};

let config = WeatherConfig {
    api_endpoint: "https://api.open-meteo.com/v1/forecast".to_string(),
    geocoding_endpoint: "https://geocoding-api.open-meteo.com/v1/search".to_string(),
    timeout_ms: 15_000,
    user_agent: "my-agent/1.0".to_string(),
};

let provider = WeatherProvider::with_config(config);
```

## Error Handling

Both providers return structured errors via `ActionResult`:

### Common Error Codes

- `INVALID_QUERY` / `INVALID_LOCATION`: Parameter validation failed
- `SEARCH_FAILED` / `WEATHER_FETCH_FAILED`: API request failed
- `TIMEOUT`: Request exceeded timeout limit (set by ActionBroker)

### Example Error Response

```rust
ActionResult {
    id: "call_123",
    status: ActionStatus::ActionError,
    output: vec![],
    error: Some(ActionError {
        code: "INVALID_QUERY".to_string(),
        message: "Query parameter cannot be empty".to_string(),
        details: Default::default(),
    }),
}
```

## Testing

Both providers include comprehensive unit tests:

```bash
# Run all provider tests
cargo test --lib providers

# Run specific provider tests
cargo test --lib web_search
cargo test --lib weather
```

## Observability

Both providers emit structured logs using the `tracing` crate:

```rust
// Web search logs
RUST_LOG=web_search=debug

// Weather logs
RUST_LOG=weather=debug

// All providers
RUST_LOG=web_search=debug,weather=debug
```

Example log output:

```
DEBUG web_search: Performing DuckDuckGo search query="rust programming" top_k=5
DEBUG weather: Geocoding location location="London"
DEBUG weather: Fetching weather data lat=51.5074 lon=-0.1278 units="celsius"
```

## Adding New Providers

To add a new capability provider:

1. Create a new file in `core/src/providers/`
2. Implement the `CapabilityProvider` trait
3. Include JSON schema in the descriptor metadata
4. Add exports to `providers/mod.rs`
5. Write unit tests
6. Update this README

Example structure:

```rust
use crate::action_broker::CapabilityProvider;
use crate::proto::{ActionCall, ActionResult, CapabilityDescriptor};
use async_trait::async_trait;

pub struct MyProvider;

#[async_trait]
impl CapabilityProvider for MyProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        // Return capability metadata with schema
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        // Implement capability logic
    }
}
```

## License

Same as the parent Loom project.
