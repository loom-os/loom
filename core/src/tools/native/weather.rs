use crate::tools::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::debug;

/// Configuration for weather provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherConfig {
    /// API endpoint (default: Open-Meteo)
    pub api_endpoint: String,
    /// Geocoding API endpoint
    pub geocoding_endpoint: String,
    /// Timeout for API requests in milliseconds
    pub timeout_ms: u64,
    /// User agent string
    pub user_agent: String,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            api_endpoint: "https://api.open-meteo.com/v1/forecast".to_string(),
            geocoding_endpoint: "https://geocoding-api.open-meteo.com/v1/search".to_string(),
            timeout_ms: 10_000,
            user_agent: "loom-agent/0.1".to_string(),
        }
    }
}

/// Geocoding response from Open-Meteo
#[derive(Debug, Deserialize)]
struct GeocodingResponse {
    results: Option<Vec<GeoLocation>>,
}

#[derive(Debug, Deserialize)]
struct GeoLocation {
    name: String,
    latitude: f64,
    longitude: f64,
    _country: Option<String>,
}

/// Weather response from Open-Meteo
#[derive(Debug, Deserialize)]
struct WeatherResponse {
    current: CurrentWeather,
}

#[derive(Debug, Deserialize)]
struct CurrentWeather {
    temperature_2m: f64,
    relative_humidity_2m: i32,
    wind_speed_10m: f64,
    weather_code: i32,
}

/// Weather capability provider
pub struct WeatherTool {
    config: WeatherConfig,
    http_client: reqwest::Client,
}

impl Default for WeatherTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WeatherTool {
    /// Create a new weather provider with default configuration
    pub fn new() -> Self {
        Self::with_config(WeatherConfig::default())
    }

    /// Create a new weather provider with custom configuration
    pub fn with_config(config: WeatherConfig) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_millis(config.timeout_ms))
            .user_agent(&config.user_agent)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            http_client,
        }
    }

    async fn get_coordinates(&self, location: &str) -> ToolResult<(f64, f64, String)> {
        let url = format!(
            "{}?name={}&count=1",
            self.config.geocoding_endpoint, location
        );

        let resp =
            self.http_client.get(&url).send().await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Geocoding request failed: {}", e))
            })?;

        if !resp.status().is_success() {
            return Err(ToolError::ExecutionFailed(format!(
                "Geocoding API error: {}",
                resp.status()
            )));
        }

        let data: GeocodingResponse = resp.json().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to parse geocoding response: {}", e))
        })?;

        if let Some(results) = data.results {
            if let Some(first) = results.first() {
                return Ok((first.latitude, first.longitude, first.name.clone()));
            }
        }

        Err(ToolError::NotFound(format!(
            "Location not found: {}",
            location
        )))
    }

    async fn get_weather(&self, lat: f64, lon: f64) -> ToolResult<CurrentWeather> {
        let url = format!(
            "{}?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code",
            self.config.api_endpoint, lat, lon
        );

        let resp =
            self.http_client.get(&url).send().await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Weather request failed: {}", e))
            })?;

        if !resp.status().is_success() {
            return Err(ToolError::ExecutionFailed(format!(
                "Weather API error: {}",
                resp.status()
            )));
        }

        let data: WeatherResponse = resp.json().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to parse weather response: {}", e))
        })?;

        Ok(data.current)
    }

    fn interpret_weather_code(code: i32) -> &'static str {
        match code {
            0 => "Clear sky",
            1..=3 => "Mainly clear, partly cloudy, and overcast",
            45 | 48 => "Fog and depositing rime fog",
            51 | 53 | 55 => "Drizzle: Light, moderate, and dense intensity",
            56 | 57 => "Freezing Drizzle: Light and dense intensity",
            61 | 63 | 65 => "Rain: Slight, moderate and heavy intensity",
            66 | 67 => "Freezing Rain: Light and heavy intensity",
            71 | 73 | 75 => "Snow fall: Slight, moderate, and heavy intensity",
            77 => "Snow grains",
            80..=82 => "Rain showers: Slight, moderate, and violent",
            85 | 86 => "Snow showers slight and heavy",
            95 => "Thunderstorm: Slight or moderate",
            96 | 99 => "Thunderstorm with slight and heavy hail",
            _ => "Unknown",
        }
    }
}

#[async_trait]
impl Tool for WeatherTool {
    fn name(&self) -> String {
        "weather:get".to_string()
    }

    fn description(&self) -> String {
        "Get current weather information for a location".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name or location (e.g. 'London', 'New York')"
                }
            },
            "required": ["location"]
        })
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        let location = arguments["location"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'location'".to_string()))?;

        debug!(target: "weather_tool", location = %location, "Fetching weather");

        let (lat, lon, name) = self.get_coordinates(location).await?;
        let weather = self.get_weather(lat, lon).await?;
        let condition = Self::interpret_weather_code(weather.weather_code);

        Ok(json!({
            "location": name,
            "temperature": weather.temperature_2m,
            "conditions": condition,
            "humidity": weather.relative_humidity_2m,
            "wind_speed": weather.wind_speed_10m,
            "units": "metric"
        }))
    }
}
