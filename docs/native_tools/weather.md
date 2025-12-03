# Weather Tool

Get current weather information for any location worldwide.

## API

Uses [Open-Meteo](https://open-meteo.com/) - a free, open-source weather API that requires no API key.

---

## weather:get

Get current weather for a location.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `location` | string | Yes | City name or location (e.g., "Tokyo", "New York") |

### Returns

```json
{
  "location": "Tokyo",
  "latitude": 35.6895,
  "longitude": 139.6917,
  "temperature": 18.5,
  "humidity": 65,
  "wind_speed": 12.3,
  "conditions": "Partly cloudy"
}
```

### Fields

| Field | Type | Unit | Description |
|-------|------|------|-------------|
| `location` | string | - | Resolved location name |
| `latitude` | float | degrees | Location latitude |
| `longitude` | float | degrees | Location longitude |
| `temperature` | float | °C | Current temperature |
| `humidity` | int | % | Relative humidity |
| `wind_speed` | float | km/h | Wind speed at 10m height |
| `conditions` | string | - | Human-readable weather description |

### Weather Codes

The API returns weather codes that are translated to descriptions:

| Code | Conditions |
|------|------------|
| 0 | Clear sky |
| 1, 2, 3 | Partly cloudy |
| 45, 48 | Foggy |
| 51, 53, 55 | Drizzle |
| 61, 63, 65 | Rain |
| 71, 73, 75 | Snow |
| 80, 81, 82 | Rain showers |
| 95, 96, 99 | Thunderstorm |

### Errors

| Error | Cause |
|-------|-------|
| `NotFound` | Location not found |
| `ExecutionFailed` | API request failed |

---

## Examples

### Basic Usage

```python
result = await ctx.tool("weather:get", {"location": "Tokyo"})
print(f"Weather in {result['location']}:")
print(f"  Temperature: {result['temperature']}°C")
print(f"  Humidity: {result['humidity']}%")
print(f"  Conditions: {result['conditions']}")
```

### With Error Handling

```python
try:
    result = await ctx.tool("weather:get", {"location": "Some Unknown Place"})
except Exception as e:
    print(f"Could not get weather: {e}")
```

### In a Cognitive Agent

```python
# The LLM can use weather:get naturally
cognitive = CognitiveAgent(
    ctx=agent._ctx,
    llm=llm,
    available_tools=["weather:get"],
)

result = await cognitive.run("What's the weather like in Paris?")
# Agent will call weather:get and incorporate results
```

---

## Configuration

### Proxy Support

If you're behind a firewall, set proxy environment variables:

```bash
export HTTPS_PROXY="http://127.0.0.1:7897"
```

### Custom Timeout

The default timeout is 10 seconds. Currently not configurable via environment variables.

---

## Limitations

1. **Location resolution**: Uses city names; may not find very small towns
2. **Current weather only**: Forecasts not currently supported
3. **Metric units**: Always returns Celsius and km/h
4. **Rate limits**: Open-Meteo has generous limits but may throttle heavy usage
