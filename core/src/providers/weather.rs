/// Weather Capability Provider
///
/// Provides weather.get capability using Open-Meteo API (free, no API key required)
/// Can be extended to support OpenWeatherMap or other services via configuration
use crate::action_broker::CapabilityProvider;
use crate::proto::{
    ActionCall, ActionError, ActionResult, ActionStatus, CapabilityDescriptor, ProviderKind,
};
use crate::{LoomError, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, warn};

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

/// Weather information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherInfo {
    pub location: String,
    pub temperature: f64,
    pub conditions: String,
    pub humidity: Option<i32>,
    pub wind_speed: Option<f64>,
    pub units: String,
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
    country: Option<String>,
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
pub struct WeatherProvider {
    config: WeatherConfig,
    http_client: reqwest::Client,
}

impl WeatherProvider {
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

    /// Geocode location name to coordinates
    async fn geocode(&self, location: &str) -> Result<(f64, f64, String)> {
        debug!(target: "weather", location=%location, "Geocoding location");

        let url = format!(
            "{}?name={}&count=1&language=en&format=json",
            self.config.geocoding_endpoint,
            urlencoding::encode(location)
        );

        let response = self.http_client.get(&url).send().await.map_err(|e| {
            warn!(target: "weather", error=%e, "Geocoding API request failed");
            LoomError::PluginError(format!("Geocoding request failed: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            warn!(target: "weather", status=%status, "Geocoding API returned error");
            return Err(LoomError::PluginError(format!(
                "Geocoding API returned status: {}",
                status
            )));
        }

        let geo_response: GeocodingResponse = response.json().await.map_err(|e| {
            warn!(target: "weather", error=%e, "Failed to parse geocoding response");
            LoomError::PluginError(format!("Failed to parse geocoding response: {}", e))
        })?;

        let location_info = geo_response
            .results
            .and_then(|mut r| r.pop())
            .ok_or_else(|| LoomError::PluginError(format!("Location not found: {}", location)))?;

        let display_name = if let Some(country) = location_info.country {
            format!("{}, {}", location_info.name, country)
        } else {
            location_info.name
        };

        Ok((
            location_info.latitude,
            location_info.longitude,
            display_name,
        ))
    }

    /// Fetch weather data for coordinates
    async fn fetch_weather(&self, lat: f64, lon: f64, units: &str) -> Result<(f64, i32, f64, i32)> {
        debug!(target: "weather", lat=%lat, lon=%lon, units=%units, "Fetching weather data");

        let temperature_unit = if units == "fahrenheit" {
            "fahrenheit"
        } else {
            "celsius"
        };
        let wind_speed_unit = if units == "fahrenheit" { "mph" } else { "kmh" };

        let url = format!(
            "{}?latitude={}&longitude={}&current=temperature_2m,relative_humidity_2m,wind_speed_10m,weather_code&temperature_unit={}&wind_speed_unit={}",
            self.config.api_endpoint,
            lat,
            lon,
            temperature_unit,
            wind_speed_unit
        );

        let response = self.http_client.get(&url).send().await.map_err(|e| {
            warn!(target: "weather", error=%e, "Weather API request failed");
            LoomError::PluginError(format!("Weather API request failed: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            warn!(target: "weather", status=%status, "Weather API returned error");
            return Err(LoomError::PluginError(format!(
                "Weather API returned status: {}",
                status
            )));
        }

        let weather_response: WeatherResponse = response.json().await.map_err(|e| {
            warn!(target: "weather", error=%e, "Failed to parse weather response");
            LoomError::PluginError(format!("Failed to parse weather response: {}", e))
        })?;

        Ok((
            weather_response.current.temperature_2m,
            weather_response.current.relative_humidity_2m,
            weather_response.current.wind_speed_10m,
            weather_response.current.weather_code,
        ))
    }

    /// Get weather for a location
    pub async fn get_weather(&self, location: &str, units: &str) -> Result<WeatherInfo> {
        let (lat, lon, display_name) = self.geocode(location).await?;
        let (temp, humidity, wind_speed, weather_code) =
            self.fetch_weather(lat, lon, units).await?;

        let conditions = weather_code_to_description(weather_code);

        Ok(WeatherInfo {
            location: display_name,
            temperature: temp,
            conditions,
            humidity: Some(humidity),
            wind_speed: Some(wind_speed),
            units: units.to_string(),
        })
    }
}

impl Default for WeatherProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CapabilityProvider for WeatherProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        let mut metadata = HashMap::new();
        metadata.insert(
            "desc".to_string(),
            "Get current weather information for a location".to_string(),
        );
        metadata.insert(
            "schema".to_string(),
            json!({
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
            })
            .to_string(),
        );

        CapabilityDescriptor {
            name: "weather.get".to_string(),
            version: "0.1.0".to_string(),
            provider: ProviderKind::ProviderNative as i32,
            metadata,
        }
    }

    async fn invoke(&self, call: ActionCall) -> Result<ActionResult> {
        let call_id = call.id.clone();

        // Parse input arguments
        let args: serde_json::Value = serde_json::from_slice(&call.payload)
            .map_err(|e| LoomError::PluginError(format!("Invalid JSON payload: {}", e)))?;

        let location = args
            .get("location")
            .and_then(|l| l.as_str())
            .ok_or_else(|| {
                LoomError::PluginError("Missing required parameter: location".to_string())
            })?;

        let units = args
            .get("units")
            .and_then(|u| u.as_str())
            .unwrap_or("celsius");

        // Validate units
        let normalized_units = match units.to_lowercase().as_str() {
            "celsius" | "c" => "celsius",
            "fahrenheit" | "f" => "fahrenheit",
            _ => "celsius", // default fallback
        };

        // Validate location
        if location.trim().is_empty() {
            return Ok(ActionResult {
                id: call_id,
                status: ActionStatus::ActionError as i32,
                output: vec![],
                error: Some(ActionError {
                    code: "INVALID_LOCATION".to_string(),
                    message: "Location parameter cannot be empty".to_string(),
                    details: Default::default(),
                }),
            });
        }

        // Fetch weather
        match self.get_weather(location, normalized_units).await {
            Ok(weather) => {
                let output = json!({
                    "location": weather.location,
                    "temperature": weather.temperature,
                    "conditions": weather.conditions,
                    "humidity": weather.humidity,
                    "wind_speed": weather.wind_speed,
                    "units": weather.units,
                });

                Ok(ActionResult {
                    id: call_id,
                    status: ActionStatus::ActionOk as i32,
                    output: serde_json::to_vec(&output)?,
                    error: None,
                })
            }
            Err(e) => Ok(ActionResult {
                id: call_id,
                status: ActionStatus::ActionError as i32,
                output: vec![],
                error: Some(ActionError {
                    code: "WEATHER_FETCH_FAILED".to_string(),
                    message: format!("Failed to get weather: {}", e),
                    details: Default::default(),
                }),
            }),
        }
    }
}

/// Convert WMO weather code to human-readable description
pub fn weather_code_to_description(code: i32) -> String {
    match code {
        0 => "clear sky",
        1 => "mainly clear",
        2 => "partly cloudy",
        3 => "overcast",
        45 | 48 => "foggy",
        51 | 53 | 55 => "drizzle",
        61 | 63 | 65 => "rain",
        71 | 73 | 75 => "snow",
        77 => "snow grains",
        80..=82 => "rain showers",
        85 | 86 => "snow showers",
        95 => "thunderstorm",
        96 | 99 => "thunderstorm with hail",
        _ => "unknown",
    }
    .to_string()
}

// Module for URL encoding
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
                ' ' => "+".to_string(),
                _ => {
                    let mut buf = [0; 4];
                    let bytes = c.encode_utf8(&mut buf).as_bytes();
                    bytes.iter().map(|b| format!("%{:02X}", b)).collect()
                }
            })
            .collect()
    }
}
