/// Web Search Capability Provider
///
/// Provides web.search capability using DuckDuckGo Instant Answer API
/// Can be extended to support other search engines via configuration
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

/// Configuration for web search provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchConfig {
    /// API endpoint (default: DuckDuckGo)
    pub api_endpoint: String,
    /// Timeout for API requests in milliseconds
    pub timeout_ms: u64,
    /// User agent string
    pub user_agent: String,
}

impl Default for WebSearchConfig {
    fn default() -> Self {
        Self {
            api_endpoint: "https://api.duckduckgo.com/".to_string(),
            timeout_ms: 10_000,
            user_agent: "loom-agent/0.1".to_string(),
        }
    }
}

/// Search result item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: Option<String>,
}

/// DuckDuckGo API response structure
#[derive(Debug, Deserialize)]
struct DuckDuckGoResponse {
    #[serde(rename = "AbstractText")]
    abstract_text: String,
    #[serde(rename = "AbstractURL")]
    abstract_url: String,
    #[serde(rename = "RelatedTopics")]
    related_topics: Vec<RelatedTopic>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RelatedTopic {
    Result {
        #[serde(rename = "Text")]
        text: String,
        #[serde(rename = "FirstURL")]
        first_url: String,
    },
    Group {
        #[serde(rename = "Topics")]
        topics: Vec<RelatedTopic>,
    },
}

/// Web search capability provider
pub struct WebSearchProvider {
    config: WebSearchConfig,
    http_client: reqwest::Client,
}

impl WebSearchProvider {
    /// Create a new web search provider with default configuration
    pub fn new() -> Self {
        Self::with_config(WebSearchConfig::default())
    }

    /// Create a new web search provider with custom configuration
    pub fn with_config(config: WebSearchConfig) -> Self {
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

    /// Perform actual search using DuckDuckGo API
    async fn search_duckduckgo(&self, query: &str, top_k: usize) -> Result<Vec<SearchResult>> {
        debug!(target: "web_search", query=%query, top_k=%top_k, "Performing DuckDuckGo search");

        let url = format!(
            "{}?q={}&format=json",
            self.config.api_endpoint,
            urlencoding::encode(query)
        );

        let response = self.http_client.get(&url).send().await.map_err(|e| {
            warn!(target: "web_search", error=%e, "DuckDuckGo API request failed");
            LoomError::PluginError(format!("Search API request failed: {}", e))
        })?;

        if !response.status().is_success() {
            let status = response.status();
            warn!(target: "web_search", status=%status, "DuckDuckGo API returned error");
            return Err(LoomError::PluginError(format!(
                "Search API returned status: {}",
                status
            )));
        }

        let ddg_response: DuckDuckGoResponse = response.json().await.map_err(|e| {
            warn!(target: "web_search", error=%e, "Failed to parse DuckDuckGo response");
            LoomError::PluginError(format!("Failed to parse search response: {}", e))
        })?;

        let mut results = Vec::new();

        // Add abstract result if available
        if !ddg_response.abstract_text.is_empty() {
            results.push(SearchResult {
                title: "Summary".to_string(),
                url: ddg_response.abstract_url.clone(),
                snippet: Some(ddg_response.abstract_text.clone()),
            });
        }

        // Extract results from related topics
        fn extract_results(topics: &[RelatedTopic], results: &mut Vec<SearchResult>, limit: usize) {
            for topic in topics {
                if results.len() >= limit {
                    break;
                }
                match topic {
                    RelatedTopic::Result { text, first_url } => {
                        if !text.is_empty() && !first_url.is_empty() {
                            results.push(SearchResult {
                                title: text.clone(),
                                url: first_url.clone(),
                                snippet: None,
                            });
                        }
                    }
                    RelatedTopic::Group { topics } => {
                        extract_results(topics, results, limit);
                    }
                }
            }
        }

        extract_results(&ddg_response.related_topics, &mut results, top_k);

        // Fallback: provide a stub result if no results found
        if results.is_empty() {
            results.push(SearchResult {
                title: format!("Search: {}", query),
                url: format!("https://duckduckgo.com/?q={}", urlencoding::encode(query)),
                snippet: Some(
                    "No detailed results available. Try refining your query.".to_string(),
                ),
            });
        }

        Ok(results)
    }
}

impl Default for WebSearchProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CapabilityProvider for WebSearchProvider {
    fn descriptor(&self) -> CapabilityDescriptor {
        let mut metadata = HashMap::new();
        metadata.insert(
            "desc".to_string(),
            "Search the web for information using DuckDuckGo".to_string(),
        );
        metadata.insert(
            "schema".to_string(),
            json!({
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
            })
            .to_string(),
        );

        CapabilityDescriptor {
            name: "web.search".to_string(),
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

        let query = args.get("query").and_then(|q| q.as_str()).ok_or_else(|| {
            LoomError::PluginError("Missing required parameter: query".to_string())
        })?;

        let top_k = args
            .get("top_k")
            .and_then(|k| k.as_u64())
            .unwrap_or(5)
            .min(10) as usize;

        // Validate query
        if query.trim().is_empty() {
            return Ok(ActionResult {
                id: call_id,
                status: ActionStatus::ActionError as i32,
                output: vec![],
                error: Some(ActionError {
                    code: "INVALID_QUERY".to_string(),
                    message: "Query parameter cannot be empty".to_string(),
                    details: Default::default(),
                }),
            });
        }

        // Perform search
        match self.search_duckduckgo(query, top_k).await {
            Ok(results) => {
                let output = json!({
                    "query": query,
                    "results": results,
                    "count": results.len(),
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
                    code: "SEARCH_FAILED".to_string(),
                    message: format!("Web search failed: {}", e),
                    details: Default::default(),
                }),
            }),
        }
    }
}

// Module for URL encoding (using percent encoding)
mod urlencoding {
    pub fn encode(s: &str) -> String {
        s.chars()
            .map(|c| match c {
                'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
                ' ' => "+".to_string(),
                _ => {
                    // Correctly percent-encode UTF-8 bytes for non-ASCII characters
                    let mut buf = [0u8; 4];
                    let bytes = c.encode_utf8(&mut buf).as_bytes();
                    bytes.iter().map(|b| format!("%{:02X}", b)).collect()
                }
            })
            .collect()
    }
}
