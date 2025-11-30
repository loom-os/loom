use crate::tools::{Tool, ToolError, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use tracing::debug;

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
pub struct WebSearchTool {
    config: WebSearchConfig,
    http_client: reqwest::Client,
}

impl Default for WebSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WebSearchTool {
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
    async fn search_duckduckgo(&self, query: &str, top_k: usize) -> ToolResult<Vec<SearchResult>> {
        debug!(target: "web_search", query=%query, top_k=%top_k, "Performing DuckDuckGo search");

        let url = format!(
            "{}?q={}&format=json&no_html=1&skip_disambig=1",
            self.config.api_endpoint, query
        );

        let resp = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Search request failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(ToolError::ExecutionFailed(format!(
                "Search API error: {}",
                resp.status()
            )));
        }

        let data: DuckDuckGoResponse = resp.json().await.map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to parse search response: {}", e))
        })?;

        let mut results = Vec::new();

        // Add abstract if available
        if !data.abstract_text.is_empty() {
            results.push(SearchResult {
                title: "Abstract".to_string(),
                url: data.abstract_url,
                snippet: Some(data.abstract_text),
            });
        }

        // Add related topics
        for topic in data.related_topics {
            match topic {
                RelatedTopic::Result { text, first_url } => {
                    // Split text into title and snippet if possible (DDG format is usually "Title - Snippet")
                    let (title, snippet) = if let Some((t, s)) = text.split_once(" - ") {
                        (t.to_string(), Some(s.to_string()))
                    } else {
                        (text.clone(), Some(text))
                    };

                    results.push(SearchResult {
                        title,
                        url: first_url,
                        snippet,
                    });
                }
                RelatedTopic::Group { topics } => {
                    for sub_topic in topics {
                        if let RelatedTopic::Result { text, first_url } = sub_topic {
                            let (title, snippet) = if let Some((t, s)) = text.split_once(" - ") {
                                (t.to_string(), Some(s.to_string()))
                            } else {
                                (text.clone(), Some(text))
                            };

                            results.push(SearchResult {
                                title,
                                url: first_url,
                                snippet,
                            });
                        }
                    }
                }
            }
        }

        // Limit results
        if results.len() > top_k {
            results.truncate(top_k);
        }

        Ok(results)
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> String {
        "web:search".to_string()
    }

    fn description(&self) -> String {
        "Search the web for information".to_string()
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results (default: 5)"
                }
            },
            "required": ["query"]
        })
    }

    async fn call(&self, arguments: Value) -> ToolResult<Value> {
        let query = arguments["query"]
            .as_str()
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'query'".to_string()))?;

        let limit = arguments["limit"].as_u64().unwrap_or(5) as usize;

        let results = self.search_duckduckgo(query, limit).await?;

        Ok(json!(results))
    }
}
