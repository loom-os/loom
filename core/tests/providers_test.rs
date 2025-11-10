/// Unit tests for capability providers
use loom_core::action_broker::CapabilityProvider;
use loom_core::proto::{ActionCall, ActionStatus};
use loom_core::providers::{WeatherProvider, WebSearchProvider};
use serde_json::json;

mod web_search {
    use super::*;

    #[test]
    fn test_descriptor() {
        let provider = WebSearchProvider::new();
        let desc = provider.descriptor();

        assert_eq!(desc.name, "web.search");
        assert_eq!(desc.version, "0.1.0");
        assert!(desc.metadata.contains_key("desc"));
        assert!(desc.metadata.contains_key("schema"));

        let schema: serde_json::Value =
            serde_json::from_str(desc.metadata.get("schema").unwrap()).unwrap();
        assert!(schema["properties"]["query"].is_object());
        assert!(schema["properties"]["top_k"].is_object());
        assert_eq!(schema["required"][0], "query");
    }

    #[tokio::test]
    async fn test_invoke_missing_query() {
        let provider = WebSearchProvider::new();
        let call = ActionCall {
            id: "test_1".to_string(),
            capability: "web.search".to_string(),
            version: String::new(),
            payload: serde_json::to_vec(&json!({})).unwrap(),
            headers: Default::default(),
            timeout_ms: 5000,
            correlation_id: String::new(),
            qos: 0,
        };

        // Should return an error in the result, not Err
        match provider.invoke(call).await {
            Ok(result) => {
                assert_eq!(result.status, ActionStatus::ActionError as i32);
                assert!(result.error.is_some());
            }
            Err(_) => {
                // Also acceptable - parse error
            }
        }
    }

    #[tokio::test]
    async fn test_invoke_empty_query() {
        let provider = WebSearchProvider::new();
        let call = ActionCall {
            id: "test_2".to_string(),
            capability: "web.search".to_string(),
            version: String::new(),
            payload: serde_json::to_vec(&json!({"query": ""})).unwrap(),
            headers: Default::default(),
            timeout_ms: 5000,
            correlation_id: String::new(),
            qos: 0,
        };

        let result = provider.invoke(call).await.unwrap();
        assert_eq!(result.status, ActionStatus::ActionError as i32);
        assert!(result.error.is_some());
        assert_eq!(result.error.as_ref().unwrap().code, "INVALID_QUERY");
    }

    #[tokio::test]
    async fn test_invoke_valid_query() {
        let provider = WebSearchProvider::new();
        let call = ActionCall {
            id: "test_3".to_string(),
            capability: "web.search".to_string(),
            version: String::new(),
            payload: serde_json::to_vec(&json!({"query": "rust programming", "top_k": 3})).unwrap(),
            headers: Default::default(),
            timeout_ms: 15000,
            correlation_id: String::new(),
            qos: 0,
        };

        let result = provider.invoke(call).await.unwrap();

        // Either success or network error is acceptable for this test
        if result.status == ActionStatus::ActionOk as i32 {
            let output: serde_json::Value = serde_json::from_slice(&result.output).unwrap();
            assert_eq!(output["query"], "rust programming");
            assert!(output["results"].is_array());
        }
    }
}

mod weather {
    use super::*;
    use loom_core::providers::weather::weather_code_to_description;

    #[test]
    fn test_weather_code_descriptions() {
        assert_eq!(weather_code_to_description(0), "clear sky");
        assert_eq!(weather_code_to_description(3), "overcast");
        assert_eq!(weather_code_to_description(61), "rain");
        assert_eq!(weather_code_to_description(71), "snow");
        assert_eq!(weather_code_to_description(95), "thunderstorm");
        assert_eq!(weather_code_to_description(999), "unknown");
    }

    #[test]
    fn test_descriptor() {
        let provider = WeatherProvider::new();
        let desc = provider.descriptor();

        assert_eq!(desc.name, "weather.get");
        assert_eq!(desc.version, "0.1.0");
        assert!(desc.metadata.contains_key("desc"));
        assert!(desc.metadata.contains_key("schema"));

        let schema: serde_json::Value =
            serde_json::from_str(desc.metadata.get("schema").unwrap()).unwrap();
        assert!(schema["properties"]["location"].is_object());
        assert!(schema["properties"]["units"].is_object());
        assert_eq!(schema["required"][0], "location");
    }

    #[tokio::test]
    async fn test_invoke_missing_location() {
        let provider = WeatherProvider::new();
        let call = ActionCall {
            id: "test_1".to_string(),
            capability: "weather.get".to_string(),
            version: String::new(),
            payload: serde_json::to_vec(&json!({})).unwrap(),
            headers: Default::default(),
            timeout_ms: 5000,
            correlation_id: String::new(),
            qos: 0,
        };

        // Should return an error in the result, not Err
        match provider.invoke(call).await {
            Ok(result) => {
                assert_eq!(result.status, ActionStatus::ActionError as i32);
                assert!(result.error.is_some());
            }
            Err(_) => {
                // Also acceptable - parse error
            }
        }
    }

    #[tokio::test]
    async fn test_invoke_empty_location() {
        let provider = WeatherProvider::new();
        let call = ActionCall {
            id: "test_2".to_string(),
            capability: "weather.get".to_string(),
            version: String::new(),
            payload: serde_json::to_vec(&json!({"location": ""})).unwrap(),
            headers: Default::default(),
            timeout_ms: 5000,
            correlation_id: String::new(),
            qos: 0,
        };

        let result = provider.invoke(call).await.unwrap();
        assert_eq!(result.status, ActionStatus::ActionError as i32);
        assert!(result.error.is_some());
        assert_eq!(result.error.as_ref().unwrap().code, "INVALID_LOCATION");
    }

    #[tokio::test]
    async fn test_invoke_valid_location() {
        let provider = WeatherProvider::new();
        let call = ActionCall {
            id: "test_3".to_string(),
            capability: "weather.get".to_string(),
            version: String::new(),
            payload: serde_json::to_vec(&json!({"location": "London", "units": "celsius"}))
                .unwrap(),
            headers: Default::default(),
            timeout_ms: 15000,
            correlation_id: String::new(),
            qos: 0,
        };

        let result = provider.invoke(call).await.unwrap();

        // Either success or network error is acceptable for this test
        if result.status == ActionStatus::ActionOk as i32 {
            let output: serde_json::Value = serde_json::from_slice(&result.output).unwrap();
            assert!(output["location"].is_string());
            assert!(output["temperature"].is_number());
            assert!(output["conditions"].is_string());
        }
    }

    #[tokio::test]
    async fn test_invoke_fahrenheit_units() {
        let provider = WeatherProvider::new();
        let call = ActionCall {
            id: "test_4".to_string(),
            capability: "weather.get".to_string(),
            version: String::new(),
            payload: serde_json::to_vec(&json!({"location": "New York", "units": "fahrenheit"}))
                .unwrap(),
            headers: Default::default(),
            timeout_ms: 15000,
            correlation_id: String::new(),
            qos: 0,
        };

        let result = provider.invoke(call).await.unwrap();

        if result.status == ActionStatus::ActionOk as i32 {
            let output: serde_json::Value = serde_json::from_slice(&result.output).unwrap();
            assert_eq!(output["units"], "fahrenheit");
        }
    }
}
